"""Check repository Markdown links and agent skill packaging."""

import os
import re
import shutil
import subprocess
from html import unescape
from pathlib import Path
from tempfile import TemporaryDirectory
from urllib.parse import unquote, urlsplit


ROOT = Path(__file__).resolve().parents[2]
SKILL_NAMES = ("zenoh-nostd-develop", "zenoh-nostd-integrate")

FENCED_CODE = re.compile(
    r"^ {0,3}(`{3,}|~{3,})(?![`~])[^\n]*\n.*?(?:^ {0,3}\1[`~]*[ \t]*$|\Z)",
    re.M | re.S,
)
INLINE_CODE = re.compile(r"(?<!`)(`+)(?!`).*?(?<!`)\1(?!`)")
DESTINATION = r"(<[^>\n]+>|(?:\\.|[^\s()<>\\]|\([^()\n]*\))+)"
LINK_PATTERNS = (
    re.compile(rf"(?<!\\)\[[^\]\n]*\]\([ \t]*{DESTINATION}(?=[\s)])"),
    re.compile(rf"^ {{0,3}}\[[^\]\n]+\]:[ \t]*{DESTINATION}(?=\s|$)", re.M),
    re.compile(r"""<(?:a|img)\b[^>]*?\s(?:href|src)\s*=\s*["']([^"']+)["']""", re.I),
)


def markdown_files():
    paths = subprocess.check_output(
        ["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard", "--", "*.md"],
        cwd=ROOT,
    )
    files = {ROOT / os.fsdecode(path) for path in paths.split(b"\0") if path}
    return sorted(path for path in files if path.is_file())


def check_frontmatter(folder):
    entry = folder / "SKILL.md"
    frontmatter = re.match(r"\A---\n(.*?)\n---\n", entry.read_text(), re.S)
    if not frontmatter:
        raise ValueError(f"{entry}: missing skill frontmatter")
    name = re.search(r"^name:[ \t]*(\S+)[ \t]*$", frontmatter[1], re.M)
    description = re.search(r"^description:[ \t]*(.+)$", frontmatter[1], re.M)
    if not name or name[1] != folder.name:
        raise ValueError(f"{entry}: skill name must match the folder name")
    if not description or not 0 < len(description[1].strip()) <= 1024:
        raise ValueError(f"{entry}: expected a description of 1–1024 characters")


def markdown_body(document):
    return FENCED_CODE.sub(lambda match: "\n" * match[0].count("\n"), document.read_text())


def check_links(documents, boundary):
    for path in documents:
        document = path.resolve()
        if not document.is_relative_to(boundary):
            raise ValueError(f"{path}: Markdown file resolves outside {boundary}")
        body = INLINE_CODE.sub("", markdown_body(document))
        for pattern in LINK_PATTERNS:
            for match in pattern.finditer(body):
                destination = match[1].removeprefix("<").removesuffix(">")
                link = unescape(re.sub(r"\\([()])", r"\1", destination))
                line = body.count("\n", 0, match.start()) + 1
                check_link(document, line, link, boundary)


def check_link(document, line, link, boundary):
    url = urlsplit(link)
    if url.scheme or url.netloc:
        return
    path = unquote(url.path)
    if path.startswith("/") and boundary == ROOT:
        target = (ROOT / path.lstrip("/")).resolve()
    else:
        target = (document.parent / path).resolve() if path else document
    if not target.is_relative_to(boundary) or not target.exists():
        raise ValueError(f"{document}:{line}: invalid local link {link}")
    if url.fragment and target.suffix.lower() == ".md":
        headings = re.findall(r"^#{1,6}[ \t]+(.+)$", markdown_body(target), re.M)
        anchors = set()
        for heading in headings:
            slug = re.sub(r"[^\w -]", "", heading.lower()).replace(" ", "-")
            anchor = slug
            index = 1
            while anchor in anchors:
                anchor = f"{slug}-{index}"
                index += 1
            anchors.add(anchor)
        if unquote(url.fragment) not in anchors:
            raise ValueError(f"{document}:{line}: missing heading for {link}")


def main():
    documents = markdown_files()
    check_links(documents, ROOT)
    for document in documents:
        if document.name == "SKILL.md":
            check_frontmatter(document.parent)

    for name in SKILL_NAMES:
        folder = ROOT / "skills" / name
        check_frontmatter(folder)
        for discovery in (".agents/skills", ".claude/skills"):
            link = ROOT / discovery / name
            if not link.is_symlink() or link.resolve() != folder:
                raise ValueError(f"{link}: expected a symlink to {folder}")

    with TemporaryDirectory() as directory:
        copied = Path(directory).resolve() / "zenoh-nostd-integrate"
        shutil.copytree(ROOT / "skills" / copied.name, copied)
        check_frontmatter(copied)
        check_links(copied.rglob("*.md"), copied)
    print(
        f"Verified local links in {len(documents)} Markdown files, skill metadata, "
        "discovery symlinks, and portable references."
    )


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        raise SystemExit(str(error)) from error
