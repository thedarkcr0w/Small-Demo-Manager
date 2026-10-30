// Copyright (c) 2026 darkcr0w. All rights reserved. Personal use permitted.
// SPDX-License-Identifier: LicenseRef-darkcr0w-Personal-Use

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ui;

use anyhow::{Context, Result};
use small_demo_manager::{audio::AudioWriter, parser, updates};
use std::{
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
};

fn cli(args: &[std::ffi::OsString]) -> Result<()> {
    let command = args[0].to_string_lossy();
    match command.as_ref() {
        "inspect" | "extract" => {
            let path = PathBuf::from(args.get(1).context("Usage: SmallDemoManager inspect <demo.dem> OR extract <demo.dem> <output-directory>")?);
            let audio = if command == "extract" {
                Some(AudioWriter::new(
                    &PathBuf::from(args.get(2).context("Output directory is required")?),
                    None,
                )?)
            } else {
                None
            };
            let parsed = parser::parse(
                &path,
                Arc::new(AtomicBool::new(false)),
                Arc::new(|_| {}),
                audio,
            )?;
            println!("{}", serde_json::to_string_pretty(&parsed.game)?);
            if command == "extract" {
                println!("Extracted {} WAV clips", parsed.clips.len());
            }
        }
        "--help" | "-h" => println!(
            "SmallDemoManager [demo.dem]\nSmallDemoManager inspect <demo.dem>\nSmallDemoManager extract <demo.dem> <output-directory>\nSDM_DATA_DIR overrides the native library directory for isolated testing."
        ),
        _ => anyhow::bail!("Unknown command: {command}"),
    }
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.first().is_some_and(|a| a == "--apply-update") {
        return updates::installer::run(&PathBuf::from(
            args.get(1).context("Missing update plan")?,
        ));
    }
    if args
        .first()
        .is_some_and(|a| matches!(a.to_str(), Some("inspect" | "extract" | "--help" | "-h")))
    {
        return cli(&args);
    }
    let restart = if args.first().is_some_and(|a| a == "--update-restart") {
        Some(updates::installer::restart_context(&PathBuf::from(
            args.get(1).context("Missing update restart plan")?,
        ))?)
    } else {
        None
    };
    let startup = match &restart {
        Some((_, demo)) => demo.clone(),
        None => args.first().map(PathBuf::from),
    };
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Small Demo Manager")
            .with_inner_size([1380.0, 860.0])
            .with_min_inner_size([1000.0, 680.0])
            .with_decorations(false)
            .with_drag_and_drop(true)
            .with_icon(app_icon()),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "Small Demo Manager",
        options,
        Box::new(move |cc| {
            let app = ui::App::new(cc, startup, restart.is_some());
            if let Some((plan, _)) = &restart {
                if !app.library_is_open() {
                    return Err("Updated app could not open the saved library".into());
                }
                updates::installer::acknowledge(plan)?;
            }
            Ok(Box::new(app))
        }),
    )
    .map_err(|e| anyhow::anyhow!("Native window: {e}"))
}

fn app_icon() -> eframe::egui::IconData {
    match image::load_from_memory_with_format(
        include_bytes!("../assets/iconApp.ico"),
        image::ImageFormat::Ico,
    ) {
        Ok(image) => {
            let image = image.to_rgba8();
            eframe::egui::IconData {
                width: image.width(),
                height: image.height(),
                rgba: image.into_raw(),
            }
        }
        Err(_) => eframe::egui::IconData::default(),
    }
}
