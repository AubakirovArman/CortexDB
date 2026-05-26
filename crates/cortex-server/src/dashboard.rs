mod behavior;
mod markup;
mod style;

pub fn html() -> String {
    let mut doc = String::with_capacity(18_000);
    doc.push_str(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>CortexDB Console</title>
    <style>
"##,
    );
    doc.push_str(style::CSS);
    doc.push_str(
        r##"
    </style>
</head>
"##,
    );
    doc.push_str(markup::BODY);
    doc.push_str(
        r##"
    <script>
"##,
    );
    doc.push_str(behavior::SCRIPT);
    doc.push_str(
        r##"
    </script>
</body>
</html>
"##,
    );
    doc
}
