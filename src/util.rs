use pkrs::model::Member;
use poise::serenity_prelude::{self as serenity};

pub(crate) fn hex_to_color(hex: Option<String>) -> serenity::Colour {
    match hex {
        Some(hex) => serenity::Colour(
            u32::from_str_radix(hex.trim_start_matches("#"), 16)
                .unwrap_or(serenity::colours::roles::DEFAULT.0),
        ),
        None => serenity::colours::roles::DEFAULT,
    }
}

pub(crate) fn get_member_name(member: &Member) -> String {
    member
        .display_name
        .to_owned()
        .unwrap_or(member.name.to_owned())
}

pub(crate) fn format_significant_duration(total_secs: u64) -> String {
    const SECS_IN_MIN: u64 = 60;
    const SECS_IN_HOUR: u64 = 60 * 60;
    const SECS_IN_DAY: u64 = 24 * 60 * 60;

    let days = total_secs / SECS_IN_DAY;
    let hours = (total_secs % SECS_IN_DAY) / SECS_IN_HOUR;
    let mins = (total_secs % SECS_IN_HOUR) / SECS_IN_MIN;
    let secs = total_secs % SECS_IN_MIN;

    if days > 0 {
        format!("{}d {}h", days, hours)
    } else if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else if mins > 0 {
        format!("{}m {}s", mins, secs)
    } else {
        format!("{}s", secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_to_color_test() {
        assert_eq!(
            hex_to_color(Some("#EEEEEE".to_string())),
            serenity::Colour::new(15658734)
        );
        assert_eq!(
            hex_to_color(Some("unparseable".to_string())),
            serenity::colours::roles::DEFAULT
        );
        assert_eq!(hex_to_color(None), serenity::colours::roles::DEFAULT);
    }

    #[test]
    fn format_significant_duration_test() {
        assert_eq!(format_significant_duration(2 * 86_400 + 4 * 3_600), "2d 4h");
        assert_eq!(format_significant_duration(5 * 3_600 + 5 * 60 + 5), "5h 5m");
        assert_eq!(format_significant_duration(20 * 60 + 1), "20m 1s");
        assert_eq!(format_significant_duration(0), "0s");
    }
}
