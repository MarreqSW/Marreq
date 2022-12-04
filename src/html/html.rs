

pub fn print_header() -> String {
    "<!DOCTYPE html>
    <html lang='en'><head>
    <title>Req Manager</title>
    </head>
    <style>
table, th, td {
  border: 1px solid black;
}
table.center {
  margin-left: auto;
  margin-right: auto;
}
</style>    
    <body>".to_string()
}

pub fn print_footer() -> String {
    "</body></html>".to_string()
}