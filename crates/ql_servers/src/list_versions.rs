use chrono::DateTime;
use ql_core::{err, json::Manifest, JsonDownloadError, ListEntry};

/// Retrieves a list of available server versions to download.
///
/// # Errors
/// If Manifest (mojang's version list)
/// - couldn't be downloaded (server error or bad internet)
/// - couldn't be parsed into JSON.
///
/// Prints an error to log if Omniarchive versions couldn't be loaded.
pub async fn list() -> Result<Vec<ListEntry>, JsonDownloadError> {
    // TODO: Allow sideloading server jars
    // In Minecraft Server ecosystem, it's more common
    // to "sideload", or provide your own custom server jars.
    //
    // This isn't common in clients because distributing
    // full-built jars is against Minecraft EULA.
    // The only use case for this in clients is for running old versions.
    // We already have that built in through Omniarchive so it's fine.
    //
    // I think this "sideloading" is allowed for servers so gotta
    // provide it somehow.

    Ok(Manifest::download()
        .await?
        .versions
        .into_iter()
        .filter_map(|n| {
            if n.id.starts_with("inf-") || n.id.starts_with("in-") || n.id.starts_with("pc-") {
                return None;
            }
            if let Some(name) = n.id.strip_prefix("c0.") {
                if name.contains("_st") || name.contains("-s") {
                    return None;
                }
                if name.starts_with("0.11")
                    || name.starts_with("0.12")
                    || name.starts_with("0.13")
                    || name.starts_with("0.14")
                    || name.starts_with("0.15")
                {
                    return None;
                }

                return Some(ListEntry {
                    name: n.id,
                    is_classic_server: true,
                });
            }
            if n.id.starts_with("a1.") {
                // Minecraft a1.0.15: Added multiplayer to alpha
                let a1_0_15 = DateTime::parse_from_rfc3339("2010-08-03T19:47:25+00:00").unwrap();
                match DateTime::parse_from_rfc3339(&n.releaseTime) {
                    Ok(dt) => {
                        if dt < a1_0_15 {
                            return None;
                        }
                    }
                    Err(e) => {
                        err!("Could not parse instance date/time: {e}");
                    }
                };
            }

            Some(ListEntry {
                name: n.id,
                is_classic_server: false,
            })
        })
        .collect())
}

/*fn convert_classic_to_real_name(classic: &str) -> &str {
    let Some(classic) = classic.strip_prefix("classic/c") else {
        return classic;
    };
    match classic {
        "1.2" => "classic/c0.0.16a",
        "1.3" => "classic/c0.0.17a",
        "1.4-1327" => "classic/c0.0.18a, c0.0.18a_01 (1)",
        "1.4-1422" => "classic/c0.0.18a, c0.0.18a_01 (2)",
        "1.4.1" => "classic/c0.0.18a_02",
        "1.5" => "classic/c0.0.19a - c0.0.19a_03",
        "1.6" => "classic/c0.0.19a_04 - c0.0.19a_06",
        "1.8" => "classic/c0.0.20a (1)",
        "1.8.1" => "classic/c0.0.20a (2)",
        "1.8.2" => "classic/c0.0.20a_01 - c0.0.23a",
        "1.8.3" | "1.9" => "classic/c0.28",
        "1.9.1" => "classic/c0.29",
        "1.10" => "classic/c0.30 (1)",
        "1.10.1" => "classic/c0.30 (2)",
        _ => classic,
    }
}

fn convert_alpha_to_real_name(alpha: &str) -> &str {
    let Some(alpha) = alpha.strip_prefix("alpha/a") else {
        return alpha;
    };
    match alpha {
        "0.1.0" => "alpha/a1.0.15",
        "0.1.1-1707" => "alpha/a1.0.16",
        "0.1.2_01" => "alpha/a1.0.16_01",
        "0.1.3" => "alpha/a1.0.16_02",
        "0.1.4" => "alpha/a1.0.17",
        "0.2.0" => "alpha/a1.1.0 (1)",
        "0.2.0_01" => "alpha/a1.1.0 (2)",
        "0.2.1" => "alpha/a1.1.1, a1.1.2",
        "0.2.2" => "alpha/a1.2.0",
        "0.2.2_01" => "alpha/a1.2.0_01, a1.2.0_02",
        "0.2.3" => "alpha/a1.2.1",
        "0.2.4" => "alpha/a1.2.2",
        "0.2.5-1004" => "alpha/a1.2.3, a1.2.3_01 (1)",
        "0.2.5-0923" => "alpha/a1.2.3, a1.2.3_01 (2)",
        "0.2.5_01" => "alpha/a1.2.3_02",
        "0.2.5_02" => "alpha/a1.2.3_04",
        "0.2.6" => "alpha/a1.2.3_05, a1.2.4 (1)",
        "0.2.6_01" => "alpha/a1.2.3_05, a1.2.4 (2)",
        "0.2.6_02" => "alpha/a1.2.4_01",
        "0.2.7" => "alpha/a1.2.5",
        "0.2.8" => "alpha/a1.2.6",
        _ => alpha,
    }
}*/
