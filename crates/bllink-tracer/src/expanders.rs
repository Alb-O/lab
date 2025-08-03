use crate::BlockExpander;
use bllink_parser::{BlendFile, Result};
use std::io::{Read, Seek};

/// Expander for Scene (SC) blocks
pub struct SceneExpander;

impl<R: Read + Seek> BlockExpander<R> for SceneExpander {
    fn expand_block(
        &self,
        block_index: usize,
        blend_file: &mut BlendFile<R>,
    ) -> Result<Vec<usize>> {
        let mut dependencies = Vec::new();

        // Read the scene block data
        let scene_data = blend_file.read_block_data(block_index)?;
        let reader = blend_file.create_field_reader(&scene_data)?;

        // Read the base list pointer (Scene.base.first)
        if let Ok(base_ptr) = reader.read_field_pointer("Scene", "base") {
            if base_ptr != 0 {
                // Find the block that contains this base
                if let Some(base_block_index) = blend_file.find_block_by_address(base_ptr) {
                    dependencies.push(base_block_index);

                    // Follow the linked list of Base objects
                    let mut current_base_ptr = base_ptr;

                    while current_base_ptr != 0 {
                        if let Some(base_index) = blend_file.find_block_by_address(current_base_ptr)
                        {
                            let base_data = blend_file.read_block_data(base_index)?;
                            let base_reader = blend_file.create_field_reader(&base_data)?;

                            // Add the object that this base references
                            if let Ok(object_ptr) = base_reader.read_field_pointer("Base", "object")
                            {
                                if object_ptr != 0 {
                                    if let Some(object_index) =
                                        blend_file.find_block_by_address(object_ptr)
                                    {
                                        dependencies.push(object_index);
                                    }
                                }
                            }

                            // Get the next base in the linked list
                            if let Ok(next_ptr) = base_reader.read_field_pointer("Base", "next") {
                                current_base_ptr = next_ptr;
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        Ok(dependencies)
    }

    fn can_handle(&self, code: &[u8; 4]) -> bool {
        code == b"SC\0\0"
    }
}

/// Expander for Object (OB) blocks
pub struct ObjectExpander;

impl<R: Read + Seek> BlockExpander<R> for ObjectExpander {
    fn expand_block(
        &self,
        block_index: usize,
        blend_file: &mut BlendFile<R>,
    ) -> Result<Vec<usize>> {
        let mut dependencies = Vec::new();

        // Read the object block data
        let object_data = blend_file.read_block_data(block_index)?;
        let reader = blend_file.create_field_reader(&object_data)?;

        // Add mesh data dependency
        if let Ok(data_ptr) = reader.read_field_pointer("Object", "data") {
            if data_ptr != 0 {
                if let Some(data_index) = blend_file.find_block_by_address(data_ptr) {
                    dependencies.push(data_index);
                }
            }
        }

        // Add material dependencies - read the materials array
        if let Ok(totcol) = reader.read_field_u32("Object", "totcol") {
            if totcol > 0 {
                if let Ok(mats_ptr) = reader.read_field_pointer("Object", "mat") {
                    if mats_ptr != 0 {
                        // Read the materials array block
                        if let Some(mats_index) = blend_file.find_block_by_address(mats_ptr) {
                            let mats_data = blend_file.read_block_data(mats_index)?;
                            let mats_reader = blend_file.create_field_reader(&mats_data)?;

                            // Read each material pointer in the array
                            for i in 0..totcol {
                                let offset = i as usize * blend_file.header.pointer_size as usize;
                                if let Ok(mat_ptr) = mats_reader.read_pointer(offset) {
                                    if mat_ptr != 0 {
                                        if let Some(mat_index) =
                                            blend_file.find_block_by_address(mat_ptr)
                                        {
                                            dependencies.push(mat_index);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(dependencies)
    }

    fn can_handle(&self, code: &[u8; 4]) -> bool {
        code == b"OB\0\0"
    }
}

/// Expander for Mesh (ME) blocks
pub struct MeshExpander;

impl<R: Read + Seek> BlockExpander<R> for MeshExpander {
    fn expand_block(
        &self,
        block_index: usize,
        blend_file: &mut BlendFile<R>,
    ) -> Result<Vec<usize>> {
        let mut dependencies = Vec::new();

        // Read the mesh block data
        let mesh_data = blend_file.read_block_data(block_index)?;
        let reader = blend_file.create_field_reader(&mesh_data)?;

        // Add material dependencies
        if let Ok(totcol) = reader.read_field_u32("Mesh", "totcol") {
            if totcol > 0 {
                if let Ok(mats_ptr) = reader.read_field_pointer("Mesh", "mat") {
                    if mats_ptr != 0 {
                        if let Some(mats_index) = blend_file.find_block_by_address(mats_ptr) {
                            let mats_data = blend_file.read_block_data(mats_index)?;
                            let mats_reader = blend_file.create_field_reader(&mats_data)?;

                            for i in 0..totcol {
                                let offset = i as usize * blend_file.header.pointer_size as usize;
                                if let Ok(mat_ptr) = mats_reader.read_pointer(offset) {
                                    if mat_ptr != 0 {
                                        if let Some(mat_index) =
                                            blend_file.find_block_by_address(mat_ptr)
                                        {
                                            dependencies.push(mat_index);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(dependencies)
    }

    fn can_handle(&self, code: &[u8; 4]) -> bool {
        code == b"ME\0\0"
    }
}

/// Expander for Collection (GR) blocks
pub struct CollectionExpander;

impl<R: Read + Seek> BlockExpander<R> for CollectionExpander {
    fn expand_block(
        &self,
        block_index: usize,
        blend_file: &mut BlendFile<R>,
    ) -> Result<Vec<usize>> {
        let mut dependencies = Vec::new();

        // Read the collection block data and extract pointers
        // Try both "Collection" (Blender 2.8+) and "Group" (older versions)
        let (gobject_ptr, children_ptr) = {
            let collection_data = blend_file.read_block_data(block_index)?;
            let reader = blend_file.create_field_reader(&collection_data)?;

            // Try "Collection" first (Blender 2.8+)
            let gobject = reader
                .read_field_pointer("Collection", "gobject")
                .or_else(|_| reader.read_field_pointer("Group", "gobject"))
                .unwrap_or(0);

            // Children field only exists in Blender 2.8+
            let children = reader
                .read_field_pointer("Collection", "children")
                .unwrap_or(0);

            (gobject, children)
        };

        // Traverse objects in this collection (gobject.first)
        if gobject_ptr != 0 {
            traverse_object_list(gobject_ptr, blend_file, &mut dependencies)?;
        }

        // Traverse child collections (children.first) - Blender 2.8+
        if children_ptr != 0 {
            traverse_children_list(children_ptr, blend_file, &mut dependencies)?;
        }

        Ok(dependencies)
    }

    fn can_handle(&self, code: &[u8; 4]) -> bool {
        code == b"GR\0\0"
    }
}

/// Traverse the linked list of CollectionObject/GroupObject items
fn traverse_object_list<R: Read + Seek>(
    first_ptr: u64,
    blend_file: &mut BlendFile<R>,
    dependencies: &mut Vec<usize>,
) -> Result<()> {
    let mut current_ptr = first_ptr;

    while current_ptr != 0 {
        if let Some(item_index) = blend_file.find_block_by_address(current_ptr) {
            // Extract object pointer and next pointer in a block
            let (object_ptr, next_ptr) = {
                let item_data = blend_file.read_block_data(item_index)?;
                let item_reader = blend_file.create_field_reader(&item_data)?;

                // Try both "CollectionObject" (Blender 2.8+) and "GroupObject" (older versions)
                let object = item_reader
                    .read_field_pointer("CollectionObject", "ob")
                    .or_else(|_| item_reader.read_field_pointer("GroupObject", "ob"))
                    .unwrap_or(0);

                let next = item_reader
                    .read_field_pointer("CollectionObject", "next")
                    .or_else(|_| item_reader.read_field_pointer("GroupObject", "next"))
                    .unwrap_or(0);

                (object, next)
            };

            // Add the object to dependencies
            if object_ptr != 0 {
                if let Some(object_index) = blend_file.find_block_by_address(object_ptr) {
                    dependencies.push(object_index);
                }
            }

            // Move to next item
            current_ptr = next_ptr;
        } else {
            break;
        }
    }

    Ok(())
}

/// Traverse the linked list of CollectionChild items and recursively expand child collections
fn traverse_children_list<R: Read + Seek>(
    first_ptr: u64,
    blend_file: &mut BlendFile<R>,
    dependencies: &mut Vec<usize>,
) -> Result<()> {
    let mut current_ptr = first_ptr;

    while current_ptr != 0 {
        if let Some(child_index) = blend_file.find_block_by_address(current_ptr) {
            // Extract the collection pointer and next pointer in a block
            let (collection_ptr, next_ptr) = {
                let child_data = blend_file.read_block_data(child_index)?;
                let child_reader = blend_file.create_field_reader(&child_data)?;

                let collection = child_reader
                    .read_field_pointer("CollectionChild", "collection")
                    .unwrap_or(0);
                let next = child_reader
                    .read_field_pointer("CollectionChild", "next")
                    .unwrap_or(0);

                (collection, next)
            };

            // Process the collection reference
            if collection_ptr != 0 {
                if let Some(collection_index) = blend_file.find_block_by_address(collection_ptr) {
                    // Recursively expand the child collection
                    let expander = CollectionExpander;
                    let child_deps = expander.expand_block(collection_index, blend_file)?;
                    dependencies.extend(child_deps);
                }
            }

            // Move to next child
            current_ptr = next_ptr;
        } else {
            break;
        }
    }

    Ok(())
}
