//! Block handling policies
//!
//! This module centralizes policies for how different block types should be treated
//! across the application, particularly for DATA block visibility and comparison.

/// Constants for block type identification
pub const DATA_BLOCK_CODE: &str = "DATA";

/// Policy for DATA block visibility
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataBlockVisibility {
    /// Show DATA blocks
    Show,
    /// Hide DATA blocks (default)
    Hide,
}

impl DataBlockVisibility {
    /// Create from a boolean flag (true = show, false = hide)
    pub fn from_flag(show_data: bool) -> Self {
        if show_data { Self::Show } else { Self::Hide }
    }

    /// Convert to boolean flag (true = show, false = hide)
    pub fn as_flag(self) -> bool {
        matches!(self, Self::Show)
    }
}

/// Check if a block code represents a DATA block
pub fn is_data_block_code(code: &str) -> bool {
    code == DATA_BLOCK_CODE
}

/// Check if a block should be visible based on policy
pub fn is_block_visible(code: &str, policy: DataBlockVisibility) -> bool {
    match policy {
        DataBlockVisibility::Show => true,
        DataBlockVisibility::Hide => !is_data_block_code(code),
    }
}

/// Policy for DATA block comparison in diff operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataBlockCompareMode {
    /// Compare DATA blocks using size-based heuristics
    SizeBased,
    /// Compare DATA blocks using content analysis
    ContentBased,
    /// Skip DATA block comparison entirely
    Skip,
}

impl Default for DataBlockCompareMode {
    fn default() -> Self {
        Self::SizeBased
    }
}

/// Determine if two DATA block sizes indicate a significant change
pub fn is_data_size_change_significant(size_before: u32, size_after: u32) -> bool {
    // Consider changes significant if size differs by more than a small threshold
    // or if one of the sizes is zero (added/removed)
    if size_before == 0 || size_after == 0 {
        return size_before != size_after;
    }

    let larger = size_before.max(size_after) as f32;
    let smaller = size_before.min(size_after) as f32;
    let ratio = smaller / larger;

    // Consider significant if size changed by more than 5%
    ratio < 0.95
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_block_identification() {
        assert!(is_data_block_code("DATA"));
        assert!(!is_data_block_code("ME"));
        assert!(!is_data_block_code("OB"));
        assert!(!is_data_block_code(""));
    }

    #[test]
    fn test_visibility_policy() {
        // Show policy allows all blocks
        assert!(is_block_visible("DATA", DataBlockVisibility::Show));
        assert!(is_block_visible("ME", DataBlockVisibility::Show));

        // Hide policy hides DATA blocks but shows others
        assert!(!is_block_visible("DATA", DataBlockVisibility::Hide));
        assert!(is_block_visible("ME", DataBlockVisibility::Hide));
    }

    #[test]
    fn test_visibility_flags() {
        assert_eq!(
            DataBlockVisibility::from_flag(true),
            DataBlockVisibility::Show
        );
        assert_eq!(
            DataBlockVisibility::from_flag(false),
            DataBlockVisibility::Hide
        );

        assert!(DataBlockVisibility::Show.as_flag());
        assert!(!DataBlockVisibility::Hide.as_flag());
    }

    #[test]
    fn test_data_size_change_significance() {
        // Same size is not significant
        assert!(!is_data_size_change_significant(1000, 1000));

        // Small changes are not significant
        assert!(!is_data_size_change_significant(1000, 980)); // 2% change
        assert!(!is_data_size_change_significant(1000, 960)); // 4% change

        // Large changes are significant
        assert!(is_data_size_change_significant(1000, 940)); // 6% change
        assert!(is_data_size_change_significant(1000, 500)); // 50% change

        // Zero sizes (added/removed) are always significant
        assert!(is_data_size_change_significant(0, 1000));
        assert!(is_data_size_change_significant(1000, 0));
        assert!(!is_data_size_change_significant(0, 0));
    }
}
