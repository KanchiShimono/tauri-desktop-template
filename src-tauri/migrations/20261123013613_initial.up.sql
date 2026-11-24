-- Timestamps are 16-byte big-endian Unix nanoseconds with the sign bit flipped.
CREATE TABLE assignees (
    id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL CHECK (
        length(trim(name)) BETWEEN 1 AND 50 AND instr(name, char(0)) = 0
    ),
    created_at BLOB NOT NULL CHECK (length(created_at) = 16),
    deleted_at BLOB CHECK (length(deleted_at) = 16)
) STRICT;

CREATE TABLE todos (
    id TEXT NOT NULL PRIMARY KEY,
    title TEXT NOT NULL CHECK (
        length(trim(title)) BETWEEN 1 AND 100 AND instr(title, char(0)) = 0
    ),
    description TEXT CHECK (
        length(description) <= 2000 AND instr(description, char(0)) = 0
    ),
    assignee_id TEXT REFERENCES assignees (id),
    due_at BLOB CHECK (length(due_at) = 16),
    created_at BLOB NOT NULL CHECK (length(created_at) = 16),
    started_at BLOB CHECK (length(started_at) = 16),
    completed_at BLOB CHECK (length(completed_at) = 16),
    archived_at BLOB CHECK (length(archived_at) = 16),
    deleted_at BLOB CHECK (length(deleted_at) = 16),
    CHECK (completed_at IS NULL OR started_at IS NOT NULL)
) STRICT;

CREATE INDEX idx_todos_undeleted_archived_at ON todos (archived_at) WHERE deleted_at IS NULL;
CREATE INDEX idx_todos_assignee_id_deleted_at ON todos (assignee_id, deleted_at);
