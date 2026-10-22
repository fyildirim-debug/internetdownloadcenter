//! Elle deneme: `cargo run -p idc-core --example media -- <adres> [indir]`
//! Araçları kurar, biçimleri listeler; `indir` verilirse en düşük çözünürlüğü indirir.

use idc_core::model::*;
use idc_core::Engine;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let url = args.get(1).cloned().unwrap_or_else(|| "https://www.youtube.com/watch?v=aqz-KE-bpKQ".into());
    let dir = std::env::temp_dir().join("idc-media-example");
    let engine = Engine::new(dir.join("cfg"));
    let mut s = engine.settings();
    s.default_save_dir = dir.join("dl").to_string_lossy().into();
    for c in &mut s.categories {
        c.save_dir = s.default_save_dir.clone();
    }
    engine.save_settings(s.clone())?;
    println!("araçlar kuruluyor…");
    let st = engine.tools().ensure(false).await?;
    println!("yt-dlp {:?} | ffmpeg {:?} | deno {:?}", st.ytdlp.version, st.ffmpeg.version, st.deno.version);
    let probe = idc_core::media::probe(engine.tools(), &s, &RequestInfo { url: url.clone(), ..Default::default() }).await?;
    println!("başlık: {} ({:?} sn, {})", probe.title, probe.duration, probe.extractor);
    for f in &probe.formats {
        println!("  {:<28} {:<14} {:>10}", f.label, f.format_id, f.filesize.map(idc_core::util::human_size).unwrap_or_default());
    }
    if args.get(2).map(|a| a == "indir").unwrap_or(false) {
        let f = probe.formats.iter().filter(|f| !f.audio_only).min_by_key(|f| f.height.unwrap_or(9999)).unwrap();
        println!("indiriliyor: {}", f.label);
        let id = engine.add(AddRequest {
            request: RequestInfo { url: url.clone(), ..Default::default() },
            file_name: Some(probe.title.clone()),
            media: Some(MediaSpec { page_url: url, format_id: f.format_id.clone(), title: Some(probe.title.clone()), ..Default::default() }),
            start: true,
            ..Default::default()
        })?;
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            let v = engine.list().into_iter().find(|v| v.id == id).unwrap();
            println!("  {:?} {} / {:?} hız {}/s", v.status, v.downloaded, v.size, idc_core::util::human_size(v.speed));
            if matches!(v.status, Status::Completed | Status::Error) {
                println!("sonuç: {:?} {:?} -> {}/{}", v.status, v.error, v.save_dir, v.file_name);
                break;
            }
        }
    }
    Ok(())
}
