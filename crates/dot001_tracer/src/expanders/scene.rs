//! Thread-safe Scene block expander
//!
//! This expander handles Scene blocks (SC) and traces dependencies to:
//! - camera: Active camera object
//! - world: World environment settings  
//! - master_collection: Root collection containing all scene objects

use crate::simple_expander;
simple_expander! {
     SceneExpander, b"SC\0\0", "Scene" => {
        single_fields: ["camera", "world", "master_collection"],
        array_fields: []
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BlockExpander;

    #[test]
    fn test_scene_expander_properties() {
        let expander = SceneExpander;
        assert_eq!(expander.block_code(), *b"SC\0\0");
        assert_eq!(expander.expander_name(), "SceneExpander");
        assert!(expander.can_handle(b"SC\0\0"));
        assert!(!expander.can_handle(b"OB\0\0"));
    }
}
