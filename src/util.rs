const DEFAULT_LIMIT: usize = 100;
const MAX_LIMIT: usize = 500;

pub fn prepare_limit_and_desc(
    limit: Option<u16>,
    desc: Option<bool>,
) -> (usize, bool) {
    (
        limit
            .and_then(|x| Some((x as usize).clamp(1, MAX_LIMIT)))
            .unwrap_or(DEFAULT_LIMIT),
        desc.unwrap_or_default(),
    )
}
