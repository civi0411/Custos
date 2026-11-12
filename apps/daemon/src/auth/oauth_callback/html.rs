pub(crate) fn render_success_html() -> &'static str {
    r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Custos - Authorization Received</title>
  <style>
    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
      background: #0f1117;
      color: #f3f4f6;
      display: flex;
      align-items: center;
      justify-content: center;
      height: 100vh;
      margin: 0;
      padding: 20px;
      box-sizing: border-box;
    }
    .card {
      max-width: 440px;
      width: 100%;
      background: #1a1d27;
      border: 1px solid #2e3346;
      border-radius: 20px;
      padding: 40px 32px;
      text-align: center;
      box-shadow: 0 20px 40px rgba(0, 0, 0, 0.6);
    }
    .icon {
      width: 56px;
      height: 56px;
      margin: 0 auto 20px auto;
      background: rgba(16, 185, 129, 0.15);
      border: 1px solid rgba(16, 185, 129, 0.3);
      border-radius: 50%;
      display: flex;
      align-items: center;
      justify-content: center;
      color: #10b981;
      font-size: 28px;
    }
    h2 {
      font-size: 20px;
      font-weight: 600;
      color: #f9fafb;
      margin: 0 0 12px 0;
    }
    p {
      font-size: 14px;
      color: #9ca3af;
      line-height: 1.5;
      margin: 0 0 24px 0;
    }
    .badge {
      display: inline-block;
      padding: 6px 14px;
      background: #252a38;
      border: 1px solid #374151;
      border-radius: 8px;
      font-size: 12px;
      font-family: monospace;
      color: #10b981;
    }
  </style>
</head>
<body>
  <div class="card">
    <div class="icon">&#10003;</div>
    <h2>Authorization Received</h2>
    <p>Custos received the authorization callback. Return to the application to check token exchange and account status.</p>
    <div class="badge">Safe to close this window</div>
  </div>
</body>
</html>"#
}

