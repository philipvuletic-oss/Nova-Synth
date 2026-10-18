//! `cargo run --release -p demo_render [-- <output dir> [patch-name-filter]]`
//! Renders every demo patch through the real plugin with the standard phrases and writes
//! WAV + spectrogram PNG + JSON report per demo, plus `summary.md` (a Markdown table).

mod analysis;
mod host;
mod patches;
mod phrases;

use std::fmt::Write as _;
use std::path::PathBuf;

fn main() {
    let mut args = std::env::args_os().skip(1);
    let out: PathBuf = args
        .next()
        .map_or_else(|| PathBuf::from("target/demos"), PathBuf::from);
    let filter = args.next().map(|f| f.to_string_lossy().to_lowercase());
    std::fs::create_dir_all(&out).expect("create output directory");
    let phrases = phrases::all();
    let mut reports = Vec::new();
    for patch in patches::PATCHES {
        if filter
            .as_ref()
            .is_some_and(|f| !patch.name.to_lowercase().contains(f))
        {
            continue;
        }
        for phrase in &phrases {
            let rendered = host::render(patch.instantiate(), &phrase.notes, phrase.seconds);
            let stem = format!(
                "{}__{}",
                patch.name.to_lowercase().replace(' ', "_"),
                phrase.name
            );
            analysis::write_wav(
                &out.join(format!("{stem}.wav")),
                &rendered.left,
                &rendered.right,
            )
            .expect("write WAV");
            analysis::write_spectrogram(
                &out.join(format!("{stem}.png")),
                &rendered.left,
                &rendered.right,
            )
            .expect("write spectrogram");
            let report = analysis::analyse(
                patch.name,
                phrase.name,
                &rendered.left,
                &rendered.right,
                &phrase.notes,
                phrase.monophonic,
                rendered.real_time_factor,
            );
            std::fs::write(
                out.join(format!("{stem}.json")),
                serde_json::to_string_pretty(&report).expect("serialise report"),
            )
            .expect("write report");
            println!(
                "{stem}: peak {:.1} dBFS, {} warning(s)",
                report.peak_dbfs,
                report.warnings.len()
            );
            reports.push(report);
        }
    }
    let mut table = String::from(
        "| Patch | Phrase | Peak dBFS | RMS dBFS | LUFS~ | Crest dB | DC | Stereo corr | Aliasing dB | Click ratio | CPU RTF | Warnings |\n|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|\n",
    );
    for r in &reports {
        let _ =
            writeln!(
            table,
            "| {} | {} | {:.1} | {:.1} | {:.1} | {:.1} | {:.5} | {:.2} | {} | {:.2} | {:.4} | {} |",
            r.patch,
            r.phrase,
            r.peak_dbfs,
            r.rms_dbfs,
            r.loudness_lufs_ish,
            r.crest_factor_db,
            r.dc_offset,
            r.stereo_correlation,
            r.aliasing_db.map_or_else(|| String::from("-"), |a| format!("{a:.1}")),
            r.click_ratio,
            r.cpu_real_time_factor,
            if r.warnings.is_empty() { String::from("-") } else { r.warnings.join("; ") },
        );
    }
    table.push_str("\nPatches:\n\n");
    for patch in patches::PATCHES {
        let _ = writeln!(table, "- **{}**: {}", patch.name, patch.description);
    }
    std::fs::write(out.join("summary.md"), &table).expect("write summary");
    println!("{table}");
}
