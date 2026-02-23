// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ReqMan

//pub mod html;
pub mod cors;

pub fn print_header() -> String {
    "<!DOCTYPE html>
    <html lang='en'><head>
    <title>Req Manager</title>
    <link rel='stylesheet' href='/static/reqman.css'>
    <link rel='icon' type='image/x-icon' href='/static/favicon.ico'>
    </head>
    <body>"
        .to_string()
}

pub fn print_footer() -> String {
    "</body></html>".to_string()
}
