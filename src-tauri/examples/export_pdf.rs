//! Prints a long HTML document to a PDF as the app's export does, from a hidden window that is never shown
//! (no Dock icon, no focus taken), then exits: `cargo run --example export_pdf -- <file.pdf>`. Of macOS.

#[cfg(target_os = "macos")]
fn main() {
    use std::path::PathBuf;
    use std::time::Duration;

    use papers_review_lib::export::print_to_pdf;
    use tauri::{ActivationPolicy, WebviewUrl, WebviewWindowBuilder};

    let out = PathBuf::from(std::env::args().nth(1).expect("usage: export_pdf <file.pdf>"));
    let mut html = String::from("<h1>Review of a manuscript</h1><p><em>Referee 1, 3 November 2026</em></p>");
    for section in 1..=6 {
        html += &format!("<h2>Comment {section}</h2>");
        for paragraph in 1..=4 {
            html += &format!(
                "<p>Paragraph {paragraph} of comment {section}. {}</p>",
                "The ice sheet responds to a warming of the ocean on timescales of centuries, and the manuscript \
                 does not say which of its two forcings drives the overshoot of the threshold. "
                    .repeat(4)
            );
        }
        html += "<blockquote>L. 214: the threshold is quoted as 1.8 K, Figure 3 shows 2.1 K.</blockquote>";
    }
    let script = format!(
        "document.body.style.font = '11pt Georgia, serif'; document.body.innerHTML = {};",
        serde_json::to_string(&html).unwrap()
    );
    // a page of a server of its own, as the app's window loads the route `/export`
    let server = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/", server.local_addr().unwrap());
    std::thread::spawn(move || {
        use std::io::Write;
        let page = "<!doctype html><meta charset=utf-8><body></body>";
        for mut stream in server.incoming().flatten() {
            let _ = write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{page}", page.len());
        }
    });
    tauri::Builder::default()
        .setup(move |app| {
            app.set_activation_policy(ActivationPolicy::Accessory);
            let window = WebviewWindowBuilder::new(app, "export-check", WebviewUrl::External(url.parse().unwrap()))
                .visible(false)
                .focused(false)
                .inner_size(595.0, 842.0)
                .build()?;
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                // the page loads and renders, as the export's page does before it calls `export_rendered`
                std::thread::sleep(Duration::from_millis(1000));
                window.eval(&script).unwrap();
                std::thread::sleep(Duration::from_millis(1000));
                let (path, shown) = (out.clone(), out.clone());
                print_to_pdf(window.as_ref(), &path, move |result| {
                    match result {
                        Ok(()) => println!("{}", shown.display()),
                        Err(e) => eprintln!("{e}"),
                    }
                    handle.exit(0);
                });
            });
            Ok(())
        })
        .build(papers_review_lib::context())
        .expect("error while building the check")
        .run(|_, _| {});
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("the PDF export is for macOS only");
}
