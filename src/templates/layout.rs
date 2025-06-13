use maud::{DOCTYPE, Markup, html};

pub fn base_layout(title: &str, content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html {
            head {
                title { (title) }
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                style {
                    "
                    :root {
                        --bg-primary: #1a1a1a;
                        --bg-secondary: #2d2d2d;
                        --text-primary: #ffffff;
                        --text-secondary: #a0a0a0;
                        --accent-success: #4ade80;
                        --accent-error: #f87171;
                        --accent-link: #60a5fa;
                    }

                    body {
                        font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
                        text-align: center;
                        padding: 0;
                        margin: 0;
                        min-height: 100vh;
                        background-color: var(--bg-primary);
                        color: var(--text-primary);
                        display: flex;
                        align-items: center;
                        justify-content: center;
                    }

                    .container {
                        width: 100%;
                        max-width: 480px;
                        margin: 20px;
                        background: var(--bg-secondary);
                        padding: 40px;
                        border-radius: 16px;
                        box-shadow: 0 4px 24px rgba(0, 0, 0, 0.2);
                    }

                    .success {
                        color: var(--accent-success);
                        font-size: 28px;
                        font-weight: 600;
                        margin-bottom: 24px;
                        letter-spacing: -0.5px;
                    }

                    .error {
                        color: var(--accent-error);
                        font-size: 28px;
                        font-weight: 600;
                        margin-bottom: 24px;
                        letter-spacing: -0.5px;
                    }

                    p {
                        color: var(--text-secondary);
                        font-size: 16px;
                        line-height: 1.6;
                        margin: 16px 0;
                    }

                    .info {
                        color: var(--text-secondary);
                        font-size: 14px;
                        margin: 24px 0;
                    }

                    .message {
                        color: var(--text-secondary);
                        font-size: 16px;
                        margin: 24px 0;
                        padding: 16px;
                        background: rgba(255, 255, 255, 0.05);
                        border-radius: 8px;
                    }

                    a {
                        color: var(--accent-link);
                        text-decoration: none;
                        font-weight: 500;
                        transition: opacity 0.2s ease;
                    }

                    a:hover {
                        opacity: 0.8;
                    }

                    strong {
                        color: var(--text-primary);
                        font-weight: 600;
                    }

                    @media (max-width: 480px) {
                        .container {
                            margin: 16px;
                            padding: 32px 24px;
                        }
                    }
                    "
                }
            }
            body {
                div class="container" {
                    (content)
                }
            }
        }
    }
}
