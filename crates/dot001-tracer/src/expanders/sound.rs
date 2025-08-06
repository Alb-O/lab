/// Modernized Sound expander using the new external_expander macro
use crate::external_expander;

external_expander! {
    SoundExpander, b"SO\0\0", "bSound" => {
        filepath_field: "filepath",
        packed_check: "packedfile"
    }
}
