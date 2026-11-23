//! Files on the core's machine: write one, read it whole or in part, and edit it in place.

use herdrify_hc::HerdrCore;

use super::BoxError;

/// Writes `notes.txt` in `dir`, reads it back, edits one line and reads part of it.
///
/// # Errors
///
/// When the core refuses a call, or the connection ends.
pub async fn write_read_edit(core: &HerdrCore, dir: &str) -> Result<(), BoxError> {
    let files = core.files();
    let path = format!("{dir}/notes.txt");

    let written = files
        .write(path.as_str(), "first line\nsecond line\nthird line\n")
        .await?;
    println!("wrote {} bytes to {path}", written.bytes);

    let whole = files.read(path.as_str()).await?;
    println!("it holds {} lines:\n{}", whole.lines, whole.text.trim_end());

    // Each edit replaces one exact piece of text; the file must contain it.
    files
        .edit(path.as_str(), [("second line", "2nd line")])
        .await?;

    // `offset` and `limit` read a range of lines.
    let part = files.read(path.as_str()).offset(2).limit(1).await?;
    println!(
        "after the edit, line {} reads {:?}",
        part.first,
        part.text.trim_end()
    );
    Ok(())
}
