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
                    body {
                        font-family: Arial, sans-serif;
                        text-align: center;
                        padding: 50px;
                        margin: 0;
                        background-color: #f5f5f5;
                    }
                    .container {
                        max-width: 600px;
                        margin: 0 auto;
                        background: white;
                        padding: 40px;
                        border-radius: 8px;
                        box-shadow: 0 2px 10px rgba(0,0,0,0.1);
                    }
                    .success {
                        color: #28a745;
                        font-size: 24px;
                        margin-bottom: 20px;
                    }
                    .error {
                        color: #dc3545;
                        font-size: 24px;
                        margin-bottom: 20px;
                    }
                    .info {
                        color: #666;
                        margin: 15px 0;
                    }
                    .message {
                        color: #666;
                        margin: 15px 0;
                    }
                    a {
                        color: #007bff;
                        text-decoration: none;
                    }
                    a:hover {
                        text-decoration: underline;
                    }
                    strong {
                        color: #333;
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
