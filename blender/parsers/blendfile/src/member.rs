use crate::error::{BlendModelError, Result};

/// Parsed array dimensions for a member name.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ArrayDims(pub Vec<usize>);

impl ArrayDims {
    pub fn len(&self) -> usize {
        self.0.iter().copied().product::<usize>().max(1)
    }
    pub fn dims(&self) -> &[usize] {
        &self.0
    }
    pub fn is_empty(&self) -> bool {
        // In DNA, absence of array dims denotes a single element (not empty). A zero-dimension
        // would represent an empty array if ever present.
        self.0.contains(&0)
    }
}

/// Member effective kind (value vs pointer, and `*` depth).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum MemberKind {
    Value,
    Pointer(u8),
}

/// Decomposed DNA member name string into parts.
/// Examples:
///  - "*next"           -> pointer depth 1, base name "next", no array
///  - "mat[4][4]"       -> value, base name "mat", dims [4,4]
///  - "**parent"        -> pointer depth 2, base name "parent"
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct MemberNameSpec {
    pub base: String,
    pub kind: MemberKind,
    pub array: ArrayDims,
}

impl MemberNameSpec {
    pub fn parse(name: &str) -> Result<Self> {
        let mut chars = name.chars().peekable();
        let mut depth = 0u8;
        while matches!(chars.peek(), Some('*')) {
            chars.next();
            depth += 1;
        }

        // read base identifier
        let mut base = String::new();
        while let Some(&c) = chars.peek() {
            if c == '[' {
                break;
            }
            base.push(c);
            chars.next();
        }
        if base.is_empty() {
            return Err(BlendModelError::InvalidMemberName(name.to_string()));
        }

        // parse array suffixes like [3][4]
        let mut dims: Vec<usize> = Vec::new();
        while matches!(chars.peek(), Some('[')) {
            chars.next(); // '['
            let mut num = String::new();
            while let Some(&c) = chars.peek() {
                if c == ']' {
                    break;
                }
                if !c.is_ascii_digit() {
                    return Err(BlendModelError::InvalidMemberName(name.to_string()));
                }
                num.push(c);
                chars.next();
            }
            if !matches!(chars.peek(), Some(']')) {
                return Err(BlendModelError::InvalidMemberName(name.to_string()));
            }
            chars.next(); // ']'
            let val: usize = num
                .parse()
                .map_err(|_| BlendModelError::InvalidMemberName(name.to_string()))?;
            dims.push(val);
        }

        Ok(Self {
            base,
            kind: if depth == 0 {
                MemberKind::Value
            } else {
                MemberKind::Pointer(depth)
            },
            array: ArrayDims(dims),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pointer_and_arrays() {
        let m = MemberNameSpec::parse("*next").unwrap();
        assert_eq!(m.base, "next");
        assert_eq!(m.kind, MemberKind::Pointer(1));
        assert!(m.array.dims().is_empty());

        let m = MemberNameSpec::parse("**parent").unwrap();
        assert_eq!(m.base, "parent");
        assert_eq!(m.kind, MemberKind::Pointer(2));

        let m = MemberNameSpec::parse("mat[4][4]").unwrap();
        assert_eq!(m.base, "mat");
        assert_eq!(m.kind, MemberKind::Value);
        assert_eq!(m.array.dims(), &[4, 4]);
        assert_eq!(m.array.len(), 16);
        assert!(!m.array.is_empty());
    }

    #[test]
    fn array_dims_semantics() {
        let a = ArrayDims(vec![]);
        assert_eq!(a.len(), 1);
        assert!(!a.is_empty());

        let a = ArrayDims(vec![3]);
        assert_eq!(a.len(), 3);
        assert!(!a.is_empty());

        let a = ArrayDims(vec![3, 4]);
        assert_eq!(a.len(), 12);
        assert!(!a.is_empty());

        let a = ArrayDims(vec![0, 4]);
        // Zero-dimension denotes empty in our semantics.
        assert!(a.is_empty());
        // len() is clamped to at least 1 for safe offset math.
        assert_eq!(a.len(), 1);
    }

    #[test]
    fn invalid_names() {
        assert!(MemberNameSpec::parse("").is_err());
        assert!(MemberNameSpec::parse("***").is_err());
        assert!(MemberNameSpec::parse("name[").is_err());
        assert!(MemberNameSpec::parse("name[xx]").is_err());
        assert!(MemberNameSpec::parse("[3]").is_err());
    }
}
