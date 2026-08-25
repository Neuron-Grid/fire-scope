use ipnet::IpNet;
use std::collections::BTreeSet;
use std::iter;

const MAX_IDENTIFIER_CHARS: usize = 64;

pub(crate) fn sanitize_identifier(input: &str) -> String {
    let sanitized = input
        .trim_matches(|character: char| !character.is_ascii_alphanumeric())
        .chars()
        .take(MAX_IDENTIFIER_CHARS)
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();

    if sanitized.is_empty() {
        "UNKNOWN".to_owned()
    } else {
        sanitized
    }
}

pub(crate) fn make_header(now: &str, country_code: &str, as_number: &str) -> String {
    format!("# Generated at: {now}\n# Country Code: {country_code}\n# AS Number: {as_number}\n\n")
}

pub(crate) fn txt_chunks<'a>(
    ipnets: &'a BTreeSet<IpNet>,
    header: &str,
) -> impl Iterator<Item = String> + 'a {
    iter::once(header.to_owned())
        .chain(ipnets.iter().map(|net| format!("{net}\n")))
        .chain(ipnets.is_empty().then(|| "\n".to_owned()))
}

pub(crate) fn nft_chunks<'a>(
    ipnets: &'a BTreeSet<IpNet>,
    header: &str,
    define_name: &str,
) -> impl Iterator<Item = String> + 'a {
    let last_index = ipnets.len().saturating_sub(1);
    iter::once(format!("{header}define {define_name} = {{\n"))
        .chain(ipnets.iter().enumerate().map(move |(index, net)| {
            let suffix = if index == last_index { "\n" } else { ",\n" };
            format!("    {net}{suffix}")
        }))
        .chain(iter::once("}\n".to_owned()))
}

#[cfg(test)]
#[path = "../tests/unit/output_render.rs"]
mod tests;
