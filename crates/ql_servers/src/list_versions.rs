use ql_core::{json::manifest::Manifest, JsonDownloadError, ListEntry};

async fn list() -> Result<Vec<ListEntry>, JsonDownloadError> {
    let manifest = Manifest::download().await?;
    let version_list: Vec<ListEntry> = manifest
        .versions
        .iter()
        .map(|n| ListEntry(n.id.clone()))
        .collect();

    Ok(version_list)
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

/// Returns a list of all available versions of the game.
pub async fn list_versions() -> Result<Vec<ListEntry>, String> {
    list().await.map_err(|n| n.to_string())
}
