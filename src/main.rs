use futures::prelude::*;
use sonor::Speaker;
use std::time::Duration;

type Result<T, E = sonor::Error> = std::result::Result<T, E>;

#[tokio::main]
async fn main() {
    loop {
        let _ = async move {
            // AssertUnwindSafe moved to the future
            std::panic::AssertUnwindSafe(discover_devices())
                .catch_unwind()
                .await
        }
        .await;

        // Wait for the next 5 second interval
        std::thread::sleep(Duration::from_secs(5));
    }
}

// Discover Sonos devices on the network
async fn discover_devices() -> Result<()> {
    let mut devices = sonor::discover(Duration::from_secs(5)).await?;

    // Create an empty vector to store the names of the speakers
    let mut device_array: Vec<String> = Vec::new();

    // Iterate over each discovered device
    while let Some(device) = devices.try_next().await? {
        let name = device.name().await?;

        // Find the Sonos speaker with the specified name
        let speaker = sonor::find(&name, Duration::from_secs(5))
            .await?
            .unwrap_or_else(|| panic!("speaker '{}' doesn't exist", name));

        println!("No new speakers to join");

        // Check if the speaker is already in a group
        if is_speaker_already_in_group(&speaker).await? {
            println!("No new speakers to join\n");
            continue;
        }

        // Attempt to find a coordinator for the speaker to join
        if let Some(coordinator_name) = get_coordinator_to_join_group(&speaker).await? {
            speaker.join(&coordinator_name).await?;
            println!("Join: {:?}\n", speaker.name().await?);
            continue;
        }

        // If there is no coordinator, add the speaker's name to the device_array
        println!("No coordinator found, collect all speakers");
        device_array.push(speaker.name().await?.clone());
    }

    // If there are speakers in the device_array, join them together into a new group
    join_them_all(device_array).await?;

    println!("");

    Ok(())
}

/// This function takes a vector of device names and creates a zone group
/// out of them by joining them all together.
async fn join_them_all(devices: Vec<String>) -> Result<()> {
    // If there are no devices to join, return early
    if devices.is_empty() {
        return Ok(());
    }

    // Find the first device in the list and make it the coordinator of the group
    let speaker = sonor::find(&devices[0], Duration::from_secs(3))
        .await?
        .unwrap_or_else(|| panic!("speaker '{}' doesn't exist", &devices[0]));

    // Join all the remaining devices to the coordinator device
    for sp in &devices[1..] {
        speaker.join(&sp).await?;
    }

    Ok(())
}

/// Checks whether the given speaker is already in a group.
///
/// Returns `true` if the speaker is already in a group, `false` otherwise.
async fn is_speaker_already_in_group(speaker: &Speaker) -> Result<bool> {
    // Get all zone group states where the number of speakers is greater than 1
    let groups: Vec<_> = speaker
        .zone_group_state()
        .await?
        .into_iter()
        .filter(|(_, speakers)| speakers.len() > 1)
        .collect();

    // If there are no groups, the speaker is not in a group
    if groups.is_empty() {
        return Ok(false);
    }

    // Check if the speaker is in any of the groups
    let found_speaker_name = speaker.name().await?;
    let is_speaker_in_group = groups
        .iter()
        .flat_map(|(_, speakers)| speakers)
        .any(|sp| sp.name() == found_speaker_name);

    Ok(is_speaker_in_group)
}

/// Attempts to find a coordinator for the speaker's zone group that the speaker can join.
/// If a coordinator is found, returns the name of the coordinator. If no coordinator is found
/// or if the speaker is already part of the group, returns `None`.
async fn get_coordinator_to_join_group(speaker: &Speaker) -> Result<Option<String>> {
    let groups: Vec<_> = speaker
        .zone_group_state() // get zone group state for the current speaker
        .await?
        .into_iter() // iterate over the groups
        .filter(|(_, speakers)| speakers.len() > 1) // filter out groups with only one speaker
        .collect();

    if groups.is_empty() {
        // if there are no groups, there is no coordinator
        return Ok(None);
    }

    let input_speaker_name = speaker.name().await?;

    let result = groups.iter().find_map(|(coordinator, speakers)| {
        let coordinator_name = speakers
            .iter()
            .find(|s| s.uuid().eq_ignore_ascii_case(coordinator))
            .expect("No coordinator for group")
            .name(); // find the coordinator for the group

        if input_speaker_name == coordinator_name {
            // if the current speaker is the coordinator, return None
            None
        } else if speakers.iter().any(|s| s.name() == input_speaker_name) {
            // if the current speaker is already in the group, return None
            None
        } else {
            // otherwise, return the coordinator name as a String
            Some(coordinator_name.to_string())
        }
    });

    Ok(result)
}
