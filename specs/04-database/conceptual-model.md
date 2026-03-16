# Conceptual Model

## Entity-Relationship Diagram

```mermaid
erDiagram
    USERS ||--o| PROFILES : "has (0..1)"
    USERS ||--o{ SESSIONS : "has (0..n)"
    USERS ||--o{ EVENTS : "hosts (0..n)"
    USERS ||--o{ REPORTS : "submits (0..n)"
    USERS ||--o{ GALLERY_IMAGES : "uploads (0..n)"

    EVENTS }o--o{ EVENT_TAGS : "tagged with"
    EVENTS ||--o{ EVENT_TAG_MAPPINGS : "has"
    EVENT_TAGS ||--o{ EVENT_TAG_MAPPINGS : "used in"

    CLUBS ||--o{ GALLERY_IMAGES : "contains (0..n)"
    CLUBS ||--o{ CLUB_MEMBERS : "has (0..n)"
    USERS ||--o{ CLUB_MEMBERS : "joins (0..n)"

    USERS {
        uuid id PK
        string discord_id UK
        string discord_username
        string avatar_url
        user_role role
        user_status status
        timestamptz created_at
        timestamptz updated_at
    }

    PROFILES {
        uuid user_id PK_FK
        string vrc_id
        string x_id
        text bio_markdown
        text bio_html
        boolean is_public
        timestamptz updated_at
    }

    SESSIONS {
        uuid id PK
        uuid user_id FK
        timestamptz expires_at
        timestamptz created_at
    }

    EVENTS {
        uuid id PK
        string external_source_id UK
        string title
        text description_markdown
        uuid host_user_id FK
        string host_name
        event_status event_status
        timestamptz start_time
        timestamptz end_time
        string location
        timestamptz created_at
        timestamptz updated_at
    }

    EVENT_TAGS {
        uuid id PK
        string name UK
        string color
    }

    EVENT_TAG_MAPPINGS {
        uuid event_id PK_FK
        uuid tag_id PK_FK
    }

    REPORTS {
        uuid id PK
        uuid reporter_user_id FK
        report_target_type target_type
        uuid target_id
        text reason
        report_status status
        timestamptz created_at
    }

    CLUBS {
        uuid id PK
        string name
        text description
        string cover_image_url
        timestamptz created_at
        timestamptz updated_at
    }

    CLUB_MEMBERS {
        uuid club_id PK_FK
        uuid user_id PK_FK
        timestamptz joined_at
    }

    GALLERY_IMAGES {
        uuid id PK
        uuid club_id FK
        uuid uploaded_by FK
        string image_url
        gallery_image_status status
        timestamptz created_at
    }
```

## Entity Descriptions

### Users
The core entity. Created on first Discord OAuth2 login. Stores Discord-provided identity (ID, username, avatar URL) and system role/status. One-to-one relationship with Profile (optional — profile may not exist until the user creates it).

### Profiles
User-editable public card. Contains VRChat ID, X/Twitter handle, Markdown bio (with server-rendered HTML), and visibility flag. Separated from Users to allow clean upsert semantics and public/private toggling.

### Sessions
Server-side login sessions. Each row represents an active browser session. Expired sessions are cleaned up by a background task. All sessions for a user are deleted on suspension (member leave).

### Events
VRChat community gatherings. Primarily created via System API (GAS sync) using `external_source_id` as the dedup key. Status lifecycle: `draft` → `published` → `cancelled` | `archived`. Host may be linked to a registered user via `host_user_id` (nullable FK).

### Event Tags
Predefined colored labels for event categorization. Many-to-many with Events via the junction table `event_tag_mappings`.

### Reports
User-submitted moderation reports targeting either a profile or an event. Duplicate reports (same reporter + same target) are rejected. New reports trigger a Discord webhook notification.

### Clubs
Sub-groups within the community. Created by Staff+. Contains a name, description, and optional cover image URL.

### Club Members
Junction table for club membership. Tracks which users belong to which clubs.

### Gallery Images
Images associated with a club. Follow an approval workflow: `pending` → `approved` | `rejected`. Only `approved` images are visible in the public gallery.
