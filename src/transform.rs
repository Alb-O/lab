use crate::view::StructView;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Default)]
pub struct Transform {
    pub loc: Option<[f32; 3]>,
    pub rot_euler: Option<[f32; 3]>,
    pub quat: Option<[f32; 4]>,
    pub scale: Option<[f32; 3]>,
}

pub fn extract_transform(view: &StructView<'_>) -> Transform {
    let loc = view
        .get_f32_array("loc")
        .and_then(|v| (v.len() >= 3).then(|| [v[0], v[1], v[2]]));
    let scale = view
        .get_f32_array("size")
        .or_else(|| view.get_f32_array("scale"))
        .and_then(|v| (v.len() >= 3).then(|| [v[0], v[1], v[2]]));
    let rot_euler = view
        .get_f32_array("rot")
        .or_else(|| view.get_f32_array("rot_euler"))
        .and_then(|v| (v.len() >= 3).then(|| [v[0], v[1], v[2]]));
    let quat = view
        .get_f32_array("quat")
        .and_then(|v| (v.len() >= 4).then(|| [v[0], v[1], v[2], v[3]]));

    Transform {
        loc,
        rot_euler,
        quat,
        scale,
    }
}
