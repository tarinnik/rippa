use std::time::Duration;

pub fn format_time(duration: &Duration) -> String {
    let total_secs = duration.as_secs();
    let secs = total_secs % 60;
    let mins = (total_secs / 60) % 60;
    let hours = total_secs / 60 / 60;

    let mut s = if hours == 0 {
        String::new()
    } else {
        format!("{}:", hours)
    };
    s.push_str(&format!("{:02}:{:02}", mins, secs));
    s
}
