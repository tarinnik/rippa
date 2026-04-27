use std::{collections::HashMap, time::Duration};

use log::debug;

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

pub fn parse_selected_titles(body: &str) -> Option<HashMap<usize, Vec<usize>>> {
    debug!("Selected titles string: {}", body);
    let mut map = HashMap::new();
    let data = body.split('&').flat_map(|s| s.split('=').next());

    for item in data {
        let stripped = item.strip_prefix("title")?;
        if !item.contains("track") {
            // Just the title, remainder should be the index
            let index = stripped.parse::<usize>().ok()?;
            map.insert(index, Vec::new());
        } else {
            let track_data = stripped
                .split("track")
                .flat_map(|s| s.parse::<usize>())
                .collect::<Vec<usize>>();
            if track_data.len() == 2 {
                let title = track_data[0];
                let track = track_data[1];
                map.get_mut(&title)?.push(track);
            } else {
                return None;
            }
        }
    }

    debug!("Selected titles map: {:?}", &map);

    Some(map)
}
