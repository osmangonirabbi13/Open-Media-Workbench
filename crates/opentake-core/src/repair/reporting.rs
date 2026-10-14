use super::recovery_review::{CheckItem, RecoveryReview, ValidationOutcome};

/// Escapes untrusted text to prevent HTML injection and XSS in generated reports.
pub fn html_escape(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#x27;"),
            _ => escaped.push(c),
        }
    }
    escaped
}

/// Sanitizes absolute filesystem paths by replacing home directories with `~`
/// to protect user privacy across Linux, macOS, and Windows.
pub fn sanitize_privacy_string(input: &str) -> String {
    let mut out = input.to_string();
    if let Some(home) = std::env::var_os("HOME") {
        let home_str = home.to_string_lossy();
        if !home_str.is_empty() {
            out = out.replace(home_str.as_ref(), "~");
        }
    }
    if let Some(userprofile) = std::env::var_os("USERPROFILE") {
        let profile_str = userprofile.to_string_lossy();
        if !profile_str.is_empty() {
            out = out.replace(profile_str.as_ref(), "~");
        }
    }
    out
}

/// Generates a sanitized, pretty-printed JSON report from a `RecoveryReview`.
pub fn generate_recovery_json_report(review: &RecoveryReview) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(review)
}

/// Generates a clean, fully self-contained, offline-readable HTML recovery report.
/// Includes an inline modern dark stylesheet and strict Content Security Policy.
pub fn generate_recovery_html_report(review: &RecoveryReview) -> String {
    let outcome_badge = match review.validation_summary.overall_outcome {
        ValidationOutcome::Passed => "<span class=\"badge badge-success\">Validation Passed</span>",
        ValidationOutcome::Warning => {
            "<span class=\"badge badge-warning\">Partial / Warnings</span>"
        }
        ValidationOutcome::Failed => "<span class=\"badge badge-danger\">Validation Failed</span>",
        ValidationOutcome::Inconclusive => {
            "<span class=\"badge badge-neutral\">Inconclusive</span>"
        }
        ValidationOutcome::NotChecked => "<span class=\"badge badge-neutral\">Not Checked</span>",
    };

    let render_checks = |checks: &[CheckItem]| -> String {
        if checks.is_empty() {
            return "<tr><td colspan=\"3\" class=\"muted\">No checks in this category.</td></tr>"
                .to_string();
        }
        checks
            .iter()
            .map(|c| {
                let badge = match c.outcome {
                    ValidationOutcome::Passed => "<span class=\"status-pass\">PASS</span>",
                    ValidationOutcome::Warning => "<span class=\"status-warn\">WARN</span>",
                    ValidationOutcome::Failed => "<span class=\"status-fail\">FAIL</span>",
                    ValidationOutcome::Inconclusive => "<span class=\"status-inc\">INCON</span>",
                    ValidationOutcome::NotChecked => "<span class=\"status-skip\">SKIP</span>",
                };
                format!(
                    "<tr><td><code>{}</code></td><td>{} <strong>{}</strong></td><td>{}</td></tr>",
                    html_escape(&c.code),
                    badge,
                    html_escape(&c.title),
                    html_escape(&c.message)
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let source_duration_display = review
        .source_summary
        .duration_seconds
        .map(|d| format!("{:.2}s", d))
        .unwrap_or_else(|| "Unknown (Missing moov)".to_string());

    let output_duration_display = review
        .output_summary
        .as_ref()
        .and_then(|o| o.duration_seconds)
        .map(|d| format!("{:.2}s", d))
        .unwrap_or_else(|| "N/A".to_string());

    let output_size_display = review
        .output_summary
        .as_ref()
        .and_then(|o| o.file_size_bytes)
        .map(|b| {
            if b > 1024 * 1024 {
                format!("{:.2} MB", b as f64 / (1024.0 * 1024.0))
            } else {
                format!("{:.1} KB", b as f64 / 1024.0)
            }
        })
        .unwrap_or_else(|| "N/A".to_string());

    let warnings_html = if review.warnings.is_empty() {
        "<p class=\"muted\">No non-fatal warnings observed.</p>".to_string()
    } else {
        let items: Vec<String> = review
            .warnings
            .iter()
            .map(|w| format!("<li>{}</li>", html_escape(&sanitize_privacy_string(w))))
            .collect();
        format!("<ul class=\"warning-list\">{}</ul>", items.join("\n"))
    };

    let limitations_html = if review.limitations.is_empty() {
        "<p class=\"muted\">No specific limitations noted.</p>".to_string()
    } else {
        let items: Vec<String> = review
            .limitations
            .iter()
            .map(|l| format!("<li>{}</li>", html_escape(l)))
            .collect();
        format!("<ul class=\"limitations-list\">{}</ul>", items.join("\n"))
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline';">
  <title>Open Media Workbench — Media Recovery Report</title>
  <style>
    :root {{
      --bg: #09090b;
      --card-bg: #18181b;
      --border: #27272a;
      --text: #f4f4f5;
      --text-muted: #a1a1aa;
      --primary: #6366f1;
      --success: #22c55e;
      --warning: #f59e0b;
      --danger: #ef4444;
    }}
    body {{
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
      background: var(--bg);
      color: var(--text);
      line-height: 1.5;
      margin: 0;
      padding: 32px 20px;
    }}
    .container {{
      max-width: 900px;
      margin: 0 auto;
    }}
    header {{
      border-bottom: 1px solid var(--border);
      padding-bottom: 20px;
      margin-bottom: 24px;
      display: flex;
      justify-content: space-between;
      align-items: flex-start;
      flex-wrap: wrap;
      gap: 16px;
    }}
    h1 {{
      font-size: 22px;
      margin: 0 0 6px 0;
      letter-spacing: -0.02em;
    }}
    .subtitle {{
      color: var(--text-muted);
      font-size: 13px;
    }}
    .badge {{
      display: inline-block;
      padding: 4px 10px;
      border-radius: 9999px;
      font-size: 12px;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.05em;
    }}
    .badge-success {{ background: rgba(34, 197, 94, 0.15); color: var(--success); border: 1px solid rgba(34, 197, 94, 0.4); }}
    .badge-warning {{ background: rgba(245, 158, 11, 0.15); color: var(--warning); border: 1px solid rgba(245, 158, 11, 0.4); }}
    .badge-danger {{ background: rgba(239, 68, 68, 0.15); color: var(--danger); border: 1px solid rgba(239, 68, 68, 0.4); }}
    .badge-neutral {{ background: rgba(161, 161, 170, 0.15); color: var(--text-muted); border: 1px solid var(--border); }}
    .card {{
      background: var(--card-bg);
      border: 1px solid var(--border);
      border-radius: 8px;
      padding: 20px;
      margin-bottom: 24px;
    }}
    h2 {{
      font-size: 15px;
      font-weight: 600;
      margin: 0 0 14px 0;
      color: var(--text);
      display: flex;
      align-items: center;
      gap: 8px;
    }}
    table {{
      width: 100%;
      border-collapse: collapse;
      font-size: 13px;
    }}
    th, td {{
      padding: 8px 12px;
      text-align: left;
      border-bottom: 1px solid var(--border);
    }}
    th {{
      color: var(--text-muted);
      font-weight: 600;
      font-size: 11px;
      text-transform: uppercase;
      letter-spacing: 0.05em;
    }}
    code {{
      font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
      font-size: 12px;
      background: #000;
      padding: 2px 6px;
      border-radius: 4px;
      border: 1px solid var(--border);
    }}
    .status-pass {{ color: var(--success); font-weight: bold; font-size: 11px; margin-right: 6px; }}
    .status-warn {{ color: var(--warning); font-weight: bold; font-size: 11px; margin-right: 6px; }}
    .status-fail {{ color: var(--danger); font-weight: bold; font-size: 11px; margin-right: 6px; }}
    .status-inc, .status-skip {{ color: var(--text-muted); font-size: 11px; margin-right: 6px; }}
    .muted {{ color: var(--text-muted); font-size: 13px; }}
    ul {{ margin: 0; padding-left: 20px; font-size: 13px; }}
    li {{ margin-bottom: 4px; }}
    .warning-list li {{ color: #fde68a; }}
    .limitations-list li {{ color: #cbd5e1; }}
    footer {{
      margin-top: 40px;
      padding-top: 16px;
      border-top: 1px solid var(--border);
      font-size: 11px;
      color: var(--text-muted);
      text-align: center;
    }}
  </style>
</head>
<body>
  <div class="container">
    <header>
      <div>
        <h1>Open Media Workbench &mdash; Recovery Report</h1>
        <div class="subtitle">Job ID: <code>{}</code> &bull; Method: <strong>{}</strong></div>
      </div>
      <div>
        {}
      </div>
    </header>

    <div class="card">
      <h2>Comparison &amp; Media Summary</h2>
      <table>
        <thead>
          <tr>
            <th>Property</th>
            <th>Original Source</th>
            <th>Recovered Candidate</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td>File Name</td>
            <td><code>{}</code></td>
            <td><code>{}</code></td>
          </tr>
          <tr>
            <td>Container Format</td>
            <td>{}</td>
            <td>{}</td>
          </tr>
          <tr>
            <td>Estimated Duration</td>
            <td>{}</td>
            <td>{}</td>
          </tr>
          <tr>
            <td>File Size</td>
            <td>N/A (Source read-only)</td>
            <td>{}</td>
          </tr>
          <tr>
            <td>Stream Count</td>
            <td>{} streams</td>
            <td>{} streams</td>
          </tr>
        </tbody>
      </table>
      <p class="muted" style="margin-top: 12px; margin-bottom: 0;">
        <strong>Duration Note:</strong> {}
      </p>
    </div>

    <div class="card">
      <h2>Validation Evidence &amp; Integrity Checks</h2>
      <p class="muted" style="margin-top: 0; margin-bottom: 12px;">
        <strong>Coverage:</strong> {}
      </p>
      <table>
        <thead>
          <tr>
            <th style="width: 25%;">Code</th>
            <th style="width: 35%;">Category &amp; Item</th>
            <th style="width: 40%;">Finding Message</th>
          </tr>
        </thead>
        <tbody>
          {}
          {}
          {}
          {}
          {}
        </tbody>
      </table>
    </div>

    <div class="card">
      <h2>Warnings &amp; Non-Fatal Anomalies</h2>
      {}
    </div>

    <div class="card">
      <h2>Verified Limitations &amp; Disclaimers</h2>
      {}
    </div>

    <footer>
      Generated by Open Media Workbench &bull; Safe local media processing &bull; Source files are never modified.
    </footer>
  </div>
</body>
</html>
"#,
        html_escape(&review.recovery_job_id),
        html_escape(&review.recovery_method),
        outcome_badge,
        html_escape(&sanitize_privacy_string(&review.source_summary.file_name)),
        review
            .output_summary
            .as_ref()
            .map(|o| html_escape(&sanitize_privacy_string(&o.file_name)))
            .unwrap_or_else(|| "N/A".to_string()),
        html_escape(&review.source_summary.container_format),
        review
            .output_summary
            .as_ref()
            .map(|o| html_escape(&o.container_format))
            .unwrap_or_else(|| "N/A".to_string()),
        source_duration_display,
        output_duration_display,
        output_size_display,
        review.source_summary.streams.len(),
        review
            .output_summary
            .as_ref()
            .map(|o| o.streams.len())
            .unwrap_or(0),
        html_escape(&review.validation_summary.duration_comparison.explanation),
        html_escape(&review.validation_summary.inspection_coverage),
        render_checks(&review.validation_summary.container_checks),
        render_checks(&review.validation_summary.stream_checks),
        render_checks(&review.validation_summary.timestamp_checks),
        render_checks(&review.validation_summary.decode_checks),
        render_checks(&review.validation_summary.audio_checks),
        warnings_html,
        limitations_html
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repair::recovery_report::*;
    use crate::repair::recovery_review::RecoveryReview;

    #[test]
    fn test_html_escape_resistance() {
        let dangerous = "<script>alert('xss')</script> & \"quotes\"";
        let escaped = html_escape(dangerous);
        assert!(!escaped.contains("<script>"));
        assert!(escaped.contains("&lt;script&gt;"));
        assert!(escaped.contains("&amp;"));
        assert!(escaped.contains("&quot;"));
    }

    #[test]
    fn test_privacy_sanitization() {
        if let Some(home) = std::env::var_os("HOME") {
            let home_str = home.to_string_lossy();
            let test_path = format!("{home_str}/Videos/damaged.mp4");
            let sanitized = sanitize_privacy_string(&test_path);
            assert!(sanitized.starts_with("~/"));
            assert!(!sanitized.contains(home_str.as_ref()));
        }
    }

    #[test]
    fn test_report_generation() {
        let rep = RecoveryReport {
            schema_version: 1,
            job_id: "test-job-99".to_string(),
            source_asset_id: "asset-1".to_string(),
            source_file_name: "test.mp4".to_string(),
            recovery_method: RecoveryMethod::LosslessRemux,
            original_container: "mp4".to_string(),
            intended_output_container: "mp4".to_string(),
            actual_output_container: "mp4".to_string(),
            input_streams: Vec::new(),
            output_streams: vec![StreamReportInfo {
                index: 0,
                stream_type: "video".to_string(),
                codec_name: "h264".to_string(),
                duration_seconds: Some(5.0),
                resolution: Some("1920x1080".to_string()),
                sample_rate: None,
                channels: None,
            }],
            processing_status: RecoveryStatus::Completed,
            validation: RecoveryValidationResult {
                passed: true,
                is_playable: true,
                validation_level: ValidationLevel::SampledDecode,
                streams_present: vec!["Video (h264)".to_string()],
                streams_expected: vec!["1 video stream(s)".to_string()],
                stream_loss_detected: false,
                output_duration_seconds: Some(5.0),
                details: "Verified ok".to_string(),
                warnings: Vec::new(),
            },
            warnings: Vec::new(),
            observed_ffmpeg_errors: Vec::new(),
            output_file_size_bytes: Some(1024 * 1024),
            output_file_path: Some("/tmp/recovered.mp4".to_string()),
            source_duration_seconds: Some(5.0),
            output_duration_seconds: Some(5.0),
            duration_difference_seconds: Some(0.0),
            started_at_epoch_ms: 1000,
            completed_at_epoch_ms: 2000,
            limitations: vec!["Sampled decode only".to_string()],
        };

        let review = RecoveryReview::from_recovery_report(&rep, true, Some("cand-1".to_string()));
        let json = generate_recovery_json_report(&review).unwrap();
        assert!(json.contains("test-job-99"));

        let html = generate_recovery_html_report(&review);
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("test-job-99"));
        assert!(html.contains("Content-Security-Policy"));
    }
}
