use futures::future::Either;
use futures::prelude::*;
use sonor::Speaker;
use sonor::SpeakerInfo;
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

async fn discover_devices() -> Result<()> {
    let mut devices = sonor::discover(Duration::from_secs(5)).await?;

    let mut device_array: Vec<String> = Vec::new();

    while let Some(device) = devices.try_next().await? {
        let name = device.name().await?;

        let speaker = sonor::find(&name, Duration::from_secs(5))
            .await?
            .unwrap_or_else(|| panic!("speaker '{}' doesn't exist", name));

        println!("No new speakers to join");

        if is_speaker_already_in_group(&speaker).await? {
            println!("No new speakers to join");
            continue;
        }

        if let Some(coordinator_name) = get_coordinator_to_join_group(&speaker).await? {
            speaker.join(&coordinator_name).await?;
            println!("Join: {:?}", speaker.name().await?);
            continue;
        }

        // there is no coordinator, collect all speakers
        println!("there is no coordinator, collect all speakers");
        device_array.push(speaker.name().await?.clone());
    }

    join_them_all(device_array).await?;

    println!("");

    Ok(())
}

async fn join_them_all(devices: Vec<String>) -> Result<()> {
    if devices.len() > 0 {
        println!("devices: {:?}", &devices);
        let speaker = sonor::find(&devices[0], Duration::from_secs(3))
            .await?
            .unwrap_or_else(|| panic!("speaker '{}' doesn't exist", &devices[0]));

        for sp in &devices[1..] {
            println!("Speaker: {:?}", &sp);
            speaker.join(&sp).await?;
        }
    }

    Ok(())
}

async fn is_speaker_already_in_group(speaker: &Speaker) -> Result<bool> {
    let found_speaker_name = speaker.name().await?;

    // let found = speaker
    //     .zone_group_state()
    //     .await?
    //     .values()
    //     .flat_map(|s| s.clone())
    //     .find(|s| s.name() == found_speaker_name)
    //     .map_or_else(|| true, |_| false);

    // println!("found: {:?}", &found);

    let groups: Vec<_> = speaker
        .zone_group_state()
        .await?
        .into_iter()
        .filter(|(_, speakers)| speakers.len() > 1)
        .collect();

    if groups.is_empty() {
        return Ok(false);
    }

    for (_, speakers) in groups {
        for speaker in speakers {
            if speaker.name() == found_speaker_name {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

async fn get_coordinator_to_join_group(speaker: &Speaker) -> Result<Option<String>> {
    let groups: Vec<_> = speaker
        .zone_group_state()
        .await?
        .into_iter()
        .filter(|(_, speakers)| speakers.len() > 1)
        .collect();

    if groups.is_empty() {
        // no group no coordinater
        return Ok(None);
    }

    let input_speaker_name = speaker.name().await?;

    for (coordinator, speakers) in groups {
        let coordinator = speakers
            .iter()
            .find(|s| s.uuid().eq_ignore_ascii_case(&coordinator))
            .expect("no coordinator for group");

        if input_speaker_name == coordinator.name() {
            // if speaker is coordinator -> noop
            return Ok(None);
        }

        return Ok(Some(coordinator.name().to_string()));
    }

    return Ok(None);
}

// async fn get_coordinator_to_join_group1(speaker: &Speaker) -> Result<Option<String>> {
//     let coordinators: Vec<_> = speaker
//         .zone_group_state()
//         .await?
//         .into_iter()
//         .filter(|(coordinator, _)| coordinator.len() > 1)
//         .map(|(coordinator, _)| coordinator)
//         .collect();

//     if coordinators.is_empty() {
//         // no group no coordinater
//         return Ok(None);
//     }

//     let speaker_name = speaker.name().await?;

//     if coordinators.len() == 1 && speaker_name == coordinators[0] {
//         // only one coordinator & same name as input speaker -> only one speaker is online
//         return Ok(None);
//     }

//     // for the case of 1 man groups find a different coordinator
//     let cod = coordinators
//         .into_iter()
//         .find(|cn| cn.to_owned() != speaker_name);

//     println!("speaker_name: {:?}", &speaker_name);
//     println!("coordinator: {:?}", &cod);

//     return Ok(cod);
// }
