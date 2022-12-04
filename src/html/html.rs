

pub fn print_header() -> String {
    "<!DOCTYPE html>
    <html lang='en'><head>
    <title>Req Manager</title>
    <style>
    table, th, td {
      border: 1px solid black;
    }
    table.center {
        margin-left: auto;
        margin-right: auto;
    }
    .AllReqs {
      border: 1px solid black;
      padding: 5px;
    }
    </style>
    </head>
    <body>".to_string()
}


pub fn print_footer() -> String {
    "</body></html>".to_string()
}