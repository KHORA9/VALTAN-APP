// This is a reference file showing how to replace the remaining query macros
// All sqlx::query_as! and sqlx::query! macros need to be replaced with dynamic queries

// Replace get_by_category
// FROM:
let documents = sqlx::query_as!(
    Document,
    r#"
    SELECT * FROM documents
    WHERE category = ? AND is_deleted = false
    ORDER BY created_at DESC
    LIMIT ? OFFSET ?
    "#,
    category,
    limit,
    offset
)

// TO:
let documents = sqlx::query_as::<_, Document>(
    r#"
    SELECT * FROM documents
    WHERE category = ? AND is_deleted = false
    ORDER BY created_at DESC
    LIMIT ? OFFSET ?
    "#
)
.bind(category)
.bind(limit)
.bind(offset)

// Similar pattern for ALL remaining query_as! macros
// And for sqlx::query! macros, remove the ! and add appropriate .bind() calls
