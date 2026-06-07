use core::ops::Deref;

const DELIMITER: u8 = b'/';
const SINGLE_WILD: u8 = b'*';

const MAX_KEYEXPR_LEN: usize = 255;

#[allow(non_camel_case_types)]
#[repr(transparent)]
#[derive(PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct keyexpr(str);

#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OwnedKeyExpr {
    data: [u8; MAX_KEYEXPR_LEN],
    len: u8,
}

impl OwnedKeyExpr {
    pub fn as_str(&self) -> &str {
        unsafe { core::str::from_utf8_unchecked(&self.data[..self.len as usize]) }
    }
}

impl Deref for OwnedKeyExpr {
    type Target = keyexpr;
    fn deref(&self) -> &keyexpr {
        keyexpr::from_str_unchecked(self.as_str())
    }
}

impl AsRef<str> for OwnedKeyExpr {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl core::fmt::Display for OwnedKeyExpr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<'a> From<&'a keyexpr> for OwnedKeyExpr {
    fn from(ke: &'a keyexpr) -> Self {
        let s = ke.as_str();
        let bytes = s.as_bytes();
        let mut data = [0u8; MAX_KEYEXPR_LEN];
        let len = bytes.len().min(MAX_KEYEXPR_LEN) as u8;
        data[..len as usize].copy_from_slice(&bytes[..len as usize]);
        OwnedKeyExpr { data, len }
    }
}

impl keyexpr {
    pub fn to_owned(&self) -> OwnedKeyExpr {
        OwnedKeyExpr::from(self)
    }
}

impl keyexpr {
    pub fn new(v: &str) -> core::result::Result<&'_ Self, crate::KeyexprError> {
        macro_rules! zbail {
            ($err:expr) => {
                $crate::zbail!($err, "str '{}' is not a valid keyexpr", v)
            };
        }

        if v.is_empty() || v.ends_with('/') {
            zbail!(crate::KeyexprError::EmptyChunk);
        }

        let bytes = v.as_bytes();

        let mut chunk_start = 0;

        let mut i = 0;
        while i < bytes.len() {
            match bytes[i] {
                c if c > b'/' && c != b'?' => i += 1,

                b'/' if i == chunk_start => zbail!(crate::KeyexprError::EmptyChunk),

                b'/' => {
                    i += 1;
                    chunk_start = i;
                }

                b'*' if i != chunk_start => zbail!(crate::KeyexprError::StarInChunk),

                b'*' => match bytes.get(i + 1) {
                    None => break,

                    Some(&b'/') => {
                        i += 2;
                        chunk_start = i;
                    }

                    Some(&b'*') => match bytes.get(i + 2) {
                        None => break,

                        Some(&b'/') if matches!(bytes.get(i + 3), Some(b'*')) => {
                            #[cold]
                            fn double_star_err(v: &str, i: usize) -> crate::KeyexprError {
                                match (v.as_bytes().get(i + 4), v.as_bytes().get(i + 5)) {
                                    (None | Some(&b'/'), _) => {
                                        crate::KeyexprError::SingleStarAfterDoubleStar
                                    }
                                    (Some(&b'*'), None | Some(&b'/')) => {
                                        crate::KeyexprError::DoubleStarAfterDoubleStar
                                    }
                                    _ => crate::KeyexprError::StarInChunk,
                                }
                            }

                            zbail!(double_star_err(v, i));
                        }

                        Some(&b'/') => {
                            i += 3;
                            chunk_start = i;
                        }

                        _ => zbail!(crate::KeyexprError::StarInChunk),
                    },

                    _ => zbail!(crate::KeyexprError::StarInChunk),
                },

                b'$' if bytes.get(i + 1) != Some(&b'*') => {
                    zbail!(crate::KeyexprError::UnboundDollar)
                }

                b'$' => match bytes.get(i + 2) {
                    Some(&b'$') => zbail!(crate::KeyexprError::DollarAfterDollar),

                    Some(&b'/') | None if i == chunk_start => {
                        zbail!(crate::KeyexprError::LoneDollarStar)
                    }

                    None => break,

                    _ => i += 2,
                },

                b'#' | b'?' => zbail!(crate::KeyexprError::SharpOrQMark),

                _ => i += 1,
            }
        }

        Ok(keyexpr::from_str_unchecked(v))
    }

    pub(crate) fn is_wild_impl(&self) -> bool {
        self.0.contains(SINGLE_WILD as char)
    }

    pub const fn as_str(&self) -> &str {
        &self.0
    }

    pub const fn from_str_unchecked(s: &str) -> &Self {
        unsafe { core::mem::transmute(s) }
    }

    pub fn from_str_autocanonize(s: &str) -> Result<OwnedKeyExpr, crate::KeyexprError> {
        if let Ok(ke) = keyexpr::new(s) {
            return Ok(ke.to_owned());
        }
        canonize_str(s)
    }
}

pub fn canonize_str(s: &str) -> Result<OwnedKeyExpr, crate::KeyexprError> {
    let bytes = s.as_bytes();
    let mut out = OwnedKeyExpr {
        data: [0u8; MAX_KEYEXPR_LEN],
        len: 0,
    };

    let mut i = 0;
    while i < bytes.len() && bytes[i] == b'/' {
        i += 1;
    }

    while i < bytes.len() {
        if bytes[i] == b'/' {
            if out.len as usize >= MAX_KEYEXPR_LEN {
                crate::zbail!(
                    crate::KeyexprError::EmptyChunk,
                    "keyexpr too long after canonization"
                );
            }
            out.data[out.len as usize] = b'/';
            out.len += 1;
            i += 1;
            while i < bytes.len() && bytes[i] == b'/' {
                i += 1;
            }
        } else {
            if out.len as usize >= MAX_KEYEXPR_LEN {
                crate::zbail!(
                    crate::KeyexprError::EmptyChunk,
                    "keyexpr too long after canonization"
                );
            }
            out.data[out.len as usize] = bytes[i];
            out.len += 1;
            i += 1;
        }
    }

    if out.len > 0 && out.data[(out.len - 1) as usize] == b'/' {
        out.len -= 1;
    }

    let result_str = out.as_str();
    if result_str.is_empty() {
        crate::zbail!(crate::KeyexprError::EmptyChunk, "empty keyexpr after canonization");
    }
    keyexpr::new(result_str)?;
    Ok(out)
}

impl core::fmt::Debug for keyexpr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "ke`{}`", self.as_ref())
    }
}

impl core::fmt::Display for keyexpr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self)
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for keyexpr {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(fmt, "ke`{}`", self.as_ref())
    }
}

impl Deref for keyexpr {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        unsafe { core::mem::transmute(self) }
    }
}

impl AsRef<str> for keyexpr {
    fn as_ref(&self) -> &str {
        self
    }
}

impl PartialEq<str> for keyexpr {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<keyexpr> for str {
    fn eq(&self, other: &keyexpr) -> bool {
        self == other.as_str()
    }
}

#[allow(non_camel_case_types)]
#[repr(transparent)]
#[derive(PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct nonwild_keyexpr(keyexpr);

impl nonwild_keyexpr {}

impl Deref for nonwild_keyexpr {
    type Target = keyexpr;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> TryFrom<&'a keyexpr> for &'a nonwild_keyexpr {
    type Error = crate::KeyexprError;
    fn try_from(v: &'a keyexpr) -> Result<Self, Self::Error> {
        if v.is_wild_impl() {
            crate::zbail!(
                crate::KeyexprError::WildChunk,
                "keyexpr '{}' contains wildcards",
                v
            );
        }

        Ok(unsafe { core::mem::transmute::<&keyexpr, &nonwild_keyexpr>(v) })
    }
}

#[repr(u8)]
enum MatchComplexity {
    NoWilds = 0,
    ChunkWildsOnly = 1,
    Dsl = 2,
}

impl keyexpr {
    fn match_complexity(&self) -> MatchComplexity {
        let mut has_wilds = false;
        for &c in self.as_bytes() {
            match c {
                b'*' => has_wilds = true,
                b'$' => return MatchComplexity::Dsl,
                _ => {}
            }
        }
        if has_wilds {
            MatchComplexity::ChunkWildsOnly
        } else {
            MatchComplexity::NoWilds
        }
    }

    pub fn includes(&self, other: &Self) -> bool {
        if self.as_str() == "**" {
            return true;
        }

        let a = self.as_bytes();
        let b = other.as_bytes();

        if let Some(prefix) = a.strip_suffix(b"/**") {
            return b.starts_with(prefix)
                && (b.len() == prefix.len() || b[prefix.len()] == b'/');
        }

        if a == b {
            return true;
        }

        if other.is_wild_impl() {
            return false;
        }

        ke_matches(a, b)
    }

    pub fn intersects(&self, other: &Self) -> bool {
        let left = self.as_bytes();
        let right = other.as_bytes();

        if left == right {
            return true;
        }

        match self.match_complexity() as u8 | other.match_complexity() as u8 {
            0 => false,
            1 => it_intersect::<false>(left, right),
            _ => it_intersect::<true>(left, right),
        }
    }
}

#[cold]
fn star_dsl_intersect(mut it1: &[u8], mut it2: &[u8]) -> bool {
    fn next(s: &[u8]) -> (u8, &[u8]) {
        (s[0], &s[1..])
    }
    while !it1.is_empty() && !it2.is_empty() {
        let (current1, advanced1) = next(it1);
        let (current2, advanced2) = next(it2);
        match (current1, current2) {
            (b'$', b'$') => {
                if advanced1.len() == 1 || advanced2.len() == 1 {
                    return true;
                }
                if star_dsl_intersect(&advanced1[1..], it2) {
                    return true;
                } else {
                    return star_dsl_intersect(it1, &advanced2[1..]);
                };
            }
            (b'$', _) => {
                if advanced1.len() == 1 {
                    return true;
                }
                if star_dsl_intersect(&advanced1[1..], it2) {
                    return true;
                }
                it2 = advanced2;
            }
            (_, b'$') => {
                if advanced2.len() == 1 {
                    return true;
                }
                if star_dsl_intersect(it1, &advanced2[1..]) {
                    return true;
                }
                it1 = advanced1;
            }
            (sub1, sub2) if sub1 == sub2 => {
                it1 = advanced1;
                it2 = advanced2;
            }
            (_, _) => return false,
        }
    }
    it1.is_empty() && it2.is_empty() || it1 == b"$*" || it2 == b"$*"
}

fn chunk_it_intersect<const STAR_DSL: bool>(it1: &[u8], it2: &[u8]) -> bool {
    it1 == b"*" || it2 == b"*" || (STAR_DSL && star_dsl_intersect(it1, it2))
}

fn chunk_intersect<const STAR_DSL: bool>(c1: &[u8], c2: &[u8]) -> bool {
    if c1 == c2 {
        return true;
    }
    if has_direct_verbatim(c1) || has_direct_verbatim(c2) {
        return false;
    }
    chunk_it_intersect::<STAR_DSL>(c1, c2)
}

fn next(s: &[u8]) -> (&[u8], &[u8]) {
    match s.iter().position(|c| *c == b'/') {
        Some(i) => (&s[..i], &s[(i + 1)..]),
        None => (s, b""),
    }
}

fn ke_matches(mut a: &[u8], mut b: &[u8]) -> bool {
    while !a.is_empty() && !b.is_empty() {
        let (a_chunk, a_rest) = next(a);
        let (b_chunk, b_rest) = next(b);
        match a_chunk {
            b"**" if a_rest.is_empty() => return true,
            b"**" => {
                if ke_matches(a_rest, b) {
                    return true;
                }
                b = b_rest;
            }
            b"*" => {
                a = a_rest;
                b = b_rest;
            }
            _ if a_chunk == b_chunk => {
                a = a_rest;
                b = b_rest;
            }
            _ => return false,
        }
    }
    a.is_empty() || a == b"**"
}

fn it_intersect<const STAR_DSL: bool>(mut it1: &[u8], mut it2: &[u8]) -> bool {
    while !it1.is_empty() && !it2.is_empty() {
        let (current1, advanced1) = next(it1);
        let (current2, advanced2) = next(it2);
        match (current1, current2) {
            (b"**", _) => {
                if advanced1.is_empty() {
                    return !has_verbatim(it2);
                }
                return (!unsafe { has_direct_verbatim_non_empty(current2) }
                    && it_intersect::<STAR_DSL>(it1, advanced2))
                    || it_intersect::<STAR_DSL>(advanced1, it2);
            }
            (_, b"**") => {
                if advanced2.is_empty() {
                    return !has_verbatim(it1);
                }
                return (!unsafe { has_direct_verbatim_non_empty(current1) }
                    && it_intersect::<STAR_DSL>(advanced1, it2))
                    || it_intersect::<STAR_DSL>(it1, advanced2);
            }
            (sub1, sub2) if chunk_intersect::<STAR_DSL>(sub1, sub2) => {
                it1 = advanced1;
                it2 = advanced2;
            }
            (_, _) => return false,
        }
    }
    (it1.is_empty() || it1 == b"**") && (it2.is_empty() || it2 == b"**")
}

fn has_direct_verbatim(x: &[u8]) -> bool {
    matches!(x, [b'@', ..])
}

fn has_verbatim(x: &[u8]) -> bool {
    x.split(|c| *c == DELIMITER).any(has_direct_verbatim)
}

unsafe fn has_direct_verbatim_non_empty(x: &[u8]) -> bool {
    if x.is_empty() {
        return false;
    }
    unsafe { *x.get_unchecked(0) == b'@' }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn includes_demo_star_includes_example() {
        let a = keyexpr::new("demo/**").unwrap();
        let b = keyexpr::new("demo/example").unwrap();
        assert!(a.includes(b));
    }

    #[test]
    fn includes_example_not_include_star() {
        let a = keyexpr::new("demo/example").unwrap();
        let b = keyexpr::new("demo/**").unwrap();
        assert!(!a.includes(b));
    }

    #[test]
    fn includes_self() {
        let a = keyexpr::new("demo/example").unwrap();
        let b = keyexpr::new("demo/example").unwrap();
        assert!(a.includes(b));
    }

    #[test]
    fn includes_double_star() {
        let a = keyexpr::new("**").unwrap();
        let b = keyexpr::new("anything/at/all").unwrap();
        assert!(a.includes(b));
    }

    #[test]
    fn includes_star_prefix_not_match_without_slash() {
        let a = keyexpr::new("demo/**").unwrap();
        let b = keyexpr::new("demo-example").unwrap();
        assert!(!a.includes(b));
    }

    #[test]
    fn from_str_autocanonize_already_canonical() {
        let ke = keyexpr::from_str_autocanonize("demo/example/test").unwrap();
        assert_eq!(ke.as_str(), "demo/example/test");
    }

    #[test]
    fn from_str_autocanonize_consecutive_slashes() {
        let ke = keyexpr::from_str_autocanonize("a//b").unwrap();
        assert_eq!(ke.as_str(), "a/b");
    }

    #[test]
    fn from_str_autocanonize_many_consecutive_slashes() {
        let ke = keyexpr::from_str_autocanonize("a///b///c").unwrap();
        assert_eq!(ke.as_str(), "a/b/c");
    }

    #[test]
    fn from_str_autocanonize_leading_slash() {
        let ke = keyexpr::from_str_autocanonize("/demo/test").unwrap();
        assert_eq!(ke.as_str(), "demo/test");
    }

    #[test]
    fn from_str_autocanonize_trailing_slash() {
        let ke = keyexpr::from_str_autocanonize("demo/test/").unwrap();
        assert_eq!(ke.as_str(), "demo/test");
    }

    #[test]
    fn from_str_autocanonize_leading_and_trailing_slash() {
        let ke = keyexpr::from_str_autocanonize("/demo/test/").unwrap();
        assert_eq!(ke.as_str(), "demo/test");
    }

    #[test]
    fn from_str_autocanonize_single_slash_errors() {
        assert!(keyexpr::from_str_autocanonize("/").is_err());
    }

    #[test]
    fn from_str_autocanonize_empty_errors() {
        assert!(keyexpr::from_str_autocanonize("").is_err());
    }

    #[test]
    fn from_str_autocanonize_only_slashes_errors() {
        assert!(keyexpr::from_str_autocanonize("///").is_err());
    }

    #[test]
    fn from_str_autocanonize_invalid_chars_errors() {
        assert!(keyexpr::from_str_autocanonize("demo/#test").is_err());
        assert!(keyexpr::from_str_autocanonize("demo/?test").is_err());
    }

    #[test]
    fn from_str_autocanonize_wildcard_still_works() {
        let ke = keyexpr::from_str_autocanonize("demo/**").unwrap();
        assert_eq!(ke.as_str(), "demo/**");
    }

    #[test]
    fn from_str_autocanonize_canonical_mixed() {
        let ke = keyexpr::from_str_autocanonize("//a//b///c//").unwrap();
        assert_eq!(ke.as_str(), "a/b/c");
    }

    #[test]
    fn owned_keyexpr_from_keyexpr() {
        let ke = keyexpr::new("demo/example").unwrap();
        let owned = OwnedKeyExpr::from(ke);
        assert_eq!(owned.as_str(), "demo/example");
        assert_eq!(owned.as_str(), ke.as_str());
    }

    #[test]
    fn owned_keyexpr_deref() {
        let ke = keyexpr::from_str_autocanonize("a/b/c").unwrap();
        let _: &keyexpr = &ke;
        assert!(ke.includes(keyexpr::new("a/b/c").unwrap()));
    }
}
