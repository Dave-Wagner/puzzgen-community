// Some older Android drivers advertise a newer shader language than their
// current GLES context accepts. Never generate above that context's version.
pub fn cap_es_shader_version(reported: u16, context: Option<(u8, u8)>) -> u16 {
    context.map_or(reported, |(major, minor)| {
        reported.min(u16::from(major) * 100 + u16::from(minor) * 10)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn adreno_308_cannot_compile_its_advertised_310_in_a_300_context() {
        assert_eq!(cap_es_shader_version(310, Some((3, 0))), 300);
    }
    #[test]
    fn preserve_supported_versions_and_never_upgrade_a_shader_language() {
        assert_eq!(cap_es_shader_version(300, Some((3, 0))), 300);
        assert_eq!(cap_es_shader_version(310, Some((3, 1))), 310);
        assert_eq!(cap_es_shader_version(320, Some((3, 2))), 320);
        assert_eq!(cap_es_shader_version(300, Some((3, 2))), 300);
        assert_eq!(cap_es_shader_version(310, None), 310);
    }
}
