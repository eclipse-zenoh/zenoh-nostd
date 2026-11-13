//! embedded-cli owns editing/history, completion, typed parsing and help.
//! Its synchronous writer only touches bounded memory; the async reader drains
//! those bytes through the application's single writer before the next input.
use core::{cell::RefCell, fmt::Write as _};
use embedded_cli::{
    cli::{Cli as EmbeddedCli, CliBuilder, CliHandle},
    command::RawCommand,
    service::{CommandProcessor, FromRaw, ProcessError},
};
use heapless::Vec;

use super::{
    Command, LINE_CAPACITY, Line,
    parser::{self, EDIT_CAPACITY, Schema},
};

pub(super) const RENDER_CAPACITY: usize = 4096;
pub(super) const HISTORY_CAPACITY: usize = 1024;
pub(super) type Render = Vec<u8, RENDER_CAPACITY>;

#[derive(Clone, Copy, Debug)]
pub(super) struct RenderError;
impl embedded_io::Error for RenderError {
    fn kind(&self) -> embedded_io::ErrorKind {
        embedded_io::ErrorKind::OutOfMemory
    }
}
#[derive(Clone, Copy)]
enum Mode {
    Output,
    Capture,
    Discard,
}
pub(super) struct RenderState {
    pub bytes: Render,
    capture: Vec<u8, { EDIT_CAPACITY + 16 }>,
    mode: Mode,
}
impl RenderState {
    pub fn new() -> Self {
        Self {
            bytes: Vec::new(),
            capture: Vec::new(),
            mode: Mode::Output,
        }
    }
}
struct Sink<'a>(&'a RefCell<RenderState>);
impl embedded_io::ErrorType for Sink<'_> {
    type Error = RenderError;
}
impl embedded_io::Write for Sink<'_> {
    fn write(&mut self, bytes: &[u8]) -> Result<usize, RenderError> {
        let mut state = self.0.borrow_mut();
        match state.mode {
            Mode::Output => state.bytes.extend_from_slice(bytes),
            Mode::Capture => state.capture.extend_from_slice(bytes),
            Mode::Discard => Ok(()),
        }
        .map_err(|_| RenderError)?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> Result<(), RenderError> {
        Ok(())
    }
}
struct Processor<'a> {
    command: &'a mut Option<Command>,
    invalid: &'a mut bool,
}
impl CommandProcessor<Sink<'_>, RenderError> for Processor<'_> {
    fn process<'a>(
        &mut self,
        cli: &mut CliHandle<'_, Sink<'_>, RenderError>,
        raw: RawCommand<'a>,
    ) -> Result<(), ProcessError<'a, RenderError>> {
        if let Err(error) = parser::validate_raw(&raw) {
            *self.invalid = true;
            writeln!(cli.writer(), "error: {error}").map_err(|_| RenderError)?;
            return Ok(());
        }
        let schema = Schema::parse(raw).map_err(|error| {
            *self.invalid = true;
            ProcessError::ParseError(error)
        })?;
        match Command::try_from(schema) {
            Ok(command) => *self.command = Some(command),
            Err(error) => {
                *self.invalid = true;
                writeln!(cli.writer(), "error: {error}").map_err(|_| RenderError)?;
            }
        }
        Ok(())
    }
}

#[derive(Default)]
pub(super) struct Frame {
    pub command: Option<Command>,
    pub completed: bool,
    pub cancelled: bool,
    pub invalid: bool,
}

// Validate complete UTF-8 scalars before passing them to embedded-cli 0.2.1:
// its input accumulator assumes continuation bytes imply a valid scalar.
#[derive(Default)]
struct Utf8 {
    bytes: [u8; 4],
    used: usize,
    expected: usize,
}
impl Utf8 {
    fn accept(&mut self, byte: u8) -> Result<Option<&[u8]>, ()> {
        if self.used == 0 {
            self.expected = match byte {
                0..=0x7f => 1,
                0xc2..=0xdf => 2,
                0xe0..=0xef => 3,
                0xf0..=0xf4 => 4,
                _ => return Err(()),
            };
        } else if !(0x80..=0xbf).contains(&byte) {
            return Err(());
        }
        self.bytes[self.used] = byte;
        self.used += 1;
        if self.used < self.expected {
            return Ok(None);
        }
        let used = self.used;
        self.used = 0;
        core::str::from_utf8(&self.bytes[..used]).map_err(|_| ())?;
        Ok(Some(&self.bytes[..used]))
    }
}

type Editor<'a> = EmbeddedCli<Sink<'a>, RenderError, [u8; EDIT_CAPACITY], [u8; HISTORY_CAPACITY]>;
pub(super) struct Frontend<'a> {
    editor: Editor<'a>,
    render: &'a RefCell<RenderState>,
    utf8: Utf8,
    discard: Option<&'static str>,
    previous_enter: Option<u8>,
    chars: usize,
    idle: bool,
}
impl<'a> Frontend<'a> {
    pub fn new(render: &'a RefCell<RenderState>) -> Result<Self, RenderError> {
        let editor = CliBuilder::default()
            .writer(Sink(render))
            .prompt("")
            .command_buffer([0; EDIT_CAPACITY])
            .history_buffer([0; HISTORY_CAPACITY])
            .build()?;
        Ok(Self {
            editor,
            render,
            utf8: Utf8::default(),
            discard: None,
            previous_enter: None,
            chars: 0,
            idle: false,
        })
    }
    fn prompt(&self) -> &'static str {
        if self.idle { "zenoh> " } else { "" }
    }
    pub fn set_idle(&mut self, idle: bool) -> Result<(), RenderError> {
        if self.idle != idle {
            self.idle = idle;
            self.render.borrow_mut().mode = Mode::Discard;
            let result = self.editor.set_prompt(self.prompt());
            self.render.borrow_mut().mode = Mode::Output;
            result?;
        }
        Ok(())
    }
    fn editor_byte(&mut self, byte: u8, frame: &mut Frame) -> Result<(), RenderError> {
        self.editor.process_byte::<Schema<'_>, _>(
            byte,
            &mut Processor {
                command: &mut frame.command,
                invalid: &mut frame.invalid,
            },
        )
    }
    fn line(&mut self) -> Result<Line, &'static str> {
        // 0.2.1 has no input accessor. Its public write API redraws the current
        // input after a clear-line sequence. Capture that redraw in memory;
        // it never reaches the terminal and does not change editor contents.
        {
            let mut state = self.render.borrow_mut();
            state.capture.clear();
            state.mode = Mode::Capture;
        }
        let result = self.editor.write(|_| Ok(()));
        self.render.borrow_mut().mode = Mode::Output;
        result.map_err(|_| "could not inspect input")?;
        let state = self.render.borrow();
        let prefix = b"\r\x1b[2K";
        let input = state
            .capture
            .strip_prefix(prefix)
            .ok_or("unexpected editor redraw")?;
        let input = input
            .strip_prefix(self.prompt().as_bytes())
            .ok_or("unexpected editor prompt")?;
        let input = core::str::from_utf8(input).map_err(|_| "invalid UTF-8; discarded")?;
        self.chars = input.chars().count();
        Line::try_from(input).map_err(|_| "line exceeds 512 bytes; discarded")
    }
    fn clear(&mut self) -> Result<(), RenderError> {
        self.render.borrow_mut().mode = Mode::Discard;
        let result = (|| {
            let mut frame = Frame::default();
            // Finish any incomplete CSI sequence. Outside a sequence this
            // inserts a sentinel, which the captured length includes and the
            // following deletion removes together with the cancelled input.
            self.editor_byte(b'C', &mut frame)?;
            let _ = self.line();
            self.render.borrow_mut().mode = Mode::Discard;
            // Move to the end first, including when cancellation occurs in the
            // middle of an edited/history line. Backspace preserves history.
            for _ in 0..self.chars {
                for b in b"\x1b[C" {
                    self.editor_byte(*b, &mut frame)?;
                }
            }
            for _ in 0..self.chars {
                self.editor_byte(8, &mut frame)?;
            }
            Ok(())
        })();
        self.render.borrow_mut().mode = Mode::Output;
        self.chars = 0;
        self.utf8 = Utf8::default();
        result
    }
    fn notice(&mut self, message: &'static str) -> Result<(), RenderError> {
        let mut sink = Sink(self.render);
        let mut writer = embedded_cli::writer::Writer::new(&mut sink);
        write!(writer, "\nerror: {message}\n").map_err(|_| RenderError)
    }
    pub fn feed(&mut self, byte: u8) -> Result<Frame, RenderError> {
        let mut frame = Frame::default();
        if byte == 3 {
            self.clear()?;
            self.discard = None;
            self.previous_enter = None;
            frame.cancelled = true;
            return Ok(frame);
        }
        if matches!(byte, b'\r' | b'\n') {
            if self.previous_enter.is_some_and(|previous| previous != byte) {
                self.previous_enter = None;
                return Ok(frame);
            }
            self.previous_enter = Some(byte);
            if self.utf8.used != 0 {
                self.discard = Some("invalid UTF-8; discarded");
            }
            let line = if let Some(error) = self.discard.take() {
                Err(error)
            } else {
                self.line()
            };
            match line
                .and_then(|line| parser::normalize(&line).map(|normalized| (line, normalized)))
            {
                Ok((line, normalized)) => {
                    if normalized.as_str() != line.as_str() {
                        self.clear()?;
                        self.render.borrow_mut().mode = Mode::Discard;
                        let result = (|| {
                            for b in normalized.bytes() {
                                self.editor_byte(b, &mut frame)?;
                            }
                            Ok(())
                        })();
                        self.render.borrow_mut().mode = Mode::Output;
                        result?;
                    }
                    // The actual schema-generated parser/help runs only now,
                    // after the whole line passed length/encoding/quote checks.
                    self.editor_byte(byte, &mut frame)?;
                    // The application shows a prompt only after its async
                    // operation finishes. Suppress the editor's immediate one.
                    let prompt = self.prompt().as_bytes();
                    let mut state = self.render.borrow_mut();
                    if state.bytes.ends_with(prompt) {
                        let length = state.bytes.len() - prompt.len();
                        state.bytes.truncate(length);
                    }
                    drop(state);
                    self.chars = 0;
                    frame.invalid |= self
                        .render
                        .borrow()
                        .bytes
                        .windows(7)
                        .any(|w| w == b"error: ");
                }
                Err(error) => {
                    self.clear()?;
                    self.notice(error)?;
                    frame.invalid = true;
                }
            }
            frame.completed = true;
            return Ok(frame);
        }
        self.previous_enter = None;
        if self.discard.is_some() {
            return Ok(frame);
        }
        // DEL is the usual terminal backspace; embedded-cli expects ASCII BS.
        let byte = if byte == 127 { 8 } else { byte };
        let accepted = match self.utf8.accept(byte) {
            Ok(Some(bytes)) => Vec::<u8, 4>::from_slice(bytes).unwrap(),
            Ok(None) => return Ok(frame),
            Err(()) => {
                self.discard = Some("invalid UTF-8; discarded");
                return Ok(frame);
            }
        };
        for b in accepted {
            self.editor_byte(b, &mut frame)?;
        }
        // The editor silently ignores a full buffer. Keep extra room for the
        // normalized spelling and reject an overlong user line before that can
        // happen, including pastes, history and tab completion.
        if let Err(error) = self.line() {
            self.discard = Some(error);
        }
        Ok(frame)
    }
}

pub(super) struct Submission {
    pub command: Option<Command>,
    pub output: Render,
    pub invalid: bool,
}
pub(super) fn parse_line(input: &str) -> Result<Submission, &'static str> {
    if input.len() > LINE_CAPACITY {
        return Err("line exceeds 512 bytes; discarded");
    }
    if input.bytes().any(|b| b < 32 || b == 127) {
        return Err("control byte in command");
    }
    let render = RefCell::new(RenderState::new());
    let mut frontend = Frontend::new(&render).map_err(|_| "CLI render buffer full")?;
    for byte in input.bytes() {
        frontend.feed(byte).map_err(|_| "CLI render buffer full")?;
        render.borrow_mut().bytes.clear(); // Process arguments do not echo.
    }
    let frame = frontend.feed(b'\n').map_err(|_| "CLI render buffer full")?;
    let output = core::mem::take(&mut render.borrow_mut().bytes);
    Ok(Submission {
        command: frame.command,
        output,
        invalid: frame.invalid,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Kind;

    fn command(input: &str) -> Command {
        let parsed = parse_line(input).unwrap();
        assert!(!parsed.invalid, "{input}: {:?}", parsed.output);
        parsed.command.unwrap()
    }
    fn feed(frontend: &mut Frontend<'_>, render: &RefCell<RenderState>, bytes: &[u8]) -> Frame {
        let mut last = Frame::default();
        for byte in bytes {
            last = frontend.feed(*byte).unwrap();
            render.borrow_mut().bytes.clear();
        }
        last
    }
    #[test]
    fn runtime_arguments_and_shell_quoting_are_preserved() {
        let cmd = command("z_pub sensors/room 'hello world' --count 3");
        assert_eq!(cmd.key, "sensors/room");
        assert_eq!(cmd.payload, "hello world");
        assert_eq!(cmd.count, Some(3));
        assert_eq!(
            command("z_get 'sensors/**' --parameters 'x=1&y=2'").parameters,
            "x=1&y=2"
        );
        assert_eq!(command("z_put a \"\"").payload, "");
        assert_eq!(
            command(r#"z_put a "space and quote: \"ok\"""#).payload,
            "space and quote: \"ok\""
        );
        assert_eq!(command(r"z_put a escaped\ space").payload, "escaped space");
        assert_eq!(command("z_put a 'héllo 🌍'").payload, "héllo 🌍");
        assert_eq!(command("z_ping").count, Some(100));
        assert!(command("z_open --listen").listen);
    }
    #[test]
    fn malformed_commands_cannot_reach_the_runner() {
        for input in [
            "z_get",
            "z_get a b",
            "z_get a --size 4",
            "z_pub a b --interval-ms 0",
            "z_ping --size 257",
            "z_get a --timeout-ms",
            "z_put a 'oops",
            "z_put a \"oops",
            "z_get a --timeout-ms 1 --timeout-ms 2",
            "z_get a//b",
            "z_put a/** x",
            "z_put '' x",
            "status unexpected",
            "z_close --unexpected",
            "quit extra",
            "z_open --listen --listen",
            "z_pub a x --count nope",
            "z_sub a --count 1",
            "z_ping --size 4294967296",
            "z_get a --timeout-ms 0",
        ] {
            let parsed = parse_line(input).unwrap();
            assert!(
                parsed.invalid && parsed.command.is_none(),
                "{input}: {:?}",
                parsed.command
            );
        }
    }
    #[test]
    fn all_generated_help_fits_and_documents_options() {
        for name in [
            "help",
            "--help",
            "-h",
            "help z_open",
            "help z_put",
            "help z_get",
            "help z_pub",
            "help z_sub",
            "help z_queryable",
            "help z_querier",
            "help z_ping",
            "help z_pong",
            "help z_pub_thr",
            "help z_sub_thr",
            "help z_broker",
            "help status",
            "help quit",
            "z_get --help",
        ] {
            let parsed = parse_line(name).unwrap();
            assert!(!parsed.invalid && parsed.command.is_none(), "{name}");
            assert!(!parsed.output.is_empty(), "{name}");
        }
        let help = parse_line("help z_get").unwrap();
        assert!(
            core::str::from_utf8(&help.output)
                .unwrap()
                .contains("--timeout-ms")
        );
        let help = parse_line("help nonexistent").unwrap();
        assert!(help.invalid && help.command.is_none());
    }
    #[test]
    fn editing_history_and_completion_use_embedded_cli() {
        let render = RefCell::new(RenderState::new());
        let mut frontend = Frontend::new(&render).unwrap();
        frontend.set_idle(true).unwrap();
        let frame = feed(&mut frontend, &render, b"z_put a ab\x1b[DX\n");
        assert_eq!(frame.command.unwrap().payload, "aXb");
        let frame = feed(&mut frontend, &render, b"\x1b[A\n");
        assert_eq!(frame.command.unwrap().payload, "aXb");
        let frame = feed(&mut frontend, &render, b"z_ope\t\n");
        assert_eq!(frame.command.unwrap().kind, Kind::Open);
        let frame = feed(&mut frontend, &render, b"z_get h\xc3\xa9\x7fi\n");
        assert_eq!(frame.command.unwrap().key, "hi");
    }
    #[test]
    fn cancellation_from_middle_of_line_keeps_history() {
        let render = RefCell::new(RenderState::new());
        let mut frontend = Frontend::new(&render).unwrap();
        feed(&mut frontend, &render, b"z_get previous\n");
        assert!(feed(&mut frontend, &render, b"z_put unsafe data\x1b[D\x1b[D\x03").cancelled);
        let frame = feed(&mut frontend, &render, b"\x1b[A\n");
        assert_eq!(frame.command.unwrap().key, "previous");
        assert!(feed(&mut frontend, &render, b"\r").command.is_none());
        for partial in [b"unsafe\x1b[\x03".as_slice(), b"unsafe\x1b\x03"] {
            assert!(feed(&mut frontend, &render, partial).cancelled);
            assert_eq!(
                feed(&mut frontend, &render, b"z_get safe\n")
                    .command
                    .unwrap()
                    .key,
                "safe"
            );
        }
    }
    #[test]
    fn oversize_and_invalid_utf8_discard_the_whole_line_then_recover() {
        let render = RefCell::new(RenderState::new());
        let mut frontend = Frontend::new(&render).unwrap();
        feed(&mut frontend, &render, b"z_put unsafe ");
        for _ in 0..513 {
            feed(&mut frontend, &render, b"x");
        }
        let frame = feed(&mut frontend, &render, b"\x7f\n");
        assert!(frame.invalid && frame.command.is_none());
        for bytes in [
            b"z_put a \xc0\xaf\n".as_slice(),
            b"z_put a \xed\xa0\x80\n",
            b"z_put a \xf4\x90\x80\x80\n",
            b"z_put a \xc3\n",
        ] {
            let frame = feed(&mut frontend, &render, bytes);
            assert!(frame.invalid && frame.command.is_none());
            assert_eq!(
                feed(&mut frontend, &render, b"z_get recovered\n")
                    .command
                    .unwrap()
                    .key,
                "recovered"
            );
        }
    }
    #[test]
    fn an_owned_command_survives_reuse_of_the_input_buffer() {
        let render = RefCell::new(RenderState::new());
        let mut frontend = Frontend::new(&render).unwrap();
        let first = feed(
            &mut frontend,
            &render,
            b"z_put initial 'retained payload'\n",
        )
        .command
        .unwrap();
        for _ in 0..300 {
            assert!(
                feed(&mut frontend, &render, b"z_get replacement\n")
                    .command
                    .is_some()
            );
        }
        assert_eq!(first.key, "initial");
        assert_eq!(first.payload, "retained payload");
    }
}
