use pkrs::model::Member;
use poise::serenity_prelude::{self as serenity};

pub(crate) fn hex_to_color(hex: Option<String>) -> serenity::Colour {
    return match hex {
        Some(hex) => serenity::Colour(
            u32::from_str_radix(hex.trim_start_matches("#"), 16)
                .unwrap_or(serenity::colours::roles::DEFAULT.0),
        ),
        None => serenity::colours::roles::DEFAULT,
    };
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
