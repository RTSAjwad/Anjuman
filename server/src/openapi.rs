//! OpenAPI document generation (Stage 2).
//!
//! The `ApiDoc` struct is assembled from `utoipa` derives: components come from
//! the `anjuman_contracts` types (which carry `#[derive(ToSchema)]` behind the
//! `openapi` feature) and paths come from `#[utoipa::path(...)]` attributes.
//!
//! This module is only for documentation generation; it does not participate
//! in request handling. It is served at `/api-docs/openapi.json` (and a Swagger
//! UI at `/api-docs`) when wired into the router.
//!
//! The many `fn ...()` items below are pure `#[utoipa::path]` carriers (never
//! invoked at runtime — their generated `__path_*` markers feed the `ApiDoc`
//! derive), hence the module-level `#[allow(dead_code)]`.
#![allow(dead_code)]

use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

use anjuman_contracts::analytics::{
    AttentionStudent, ClassAnalytics, ClassCard, DailyPoint, DashboardResponse, DifficultCard,
    StudentDetail, StudentStats, StudentStatsWithEmail,
};
use anjuman_contracts::auth::{LoginRequest, LoginResponse, UserResponse};
use anjuman_contracts::cards::{
    CardBrowserPage, CardBrowserResponse, CardModResponse, MoveCardBody, MoveCardResponse,
    NoteModResponse, RescheduleBody,
};
use anjuman_contracts::classes::{
    AddMember, ClassResponse, CreateClass, MemberResponse, RenameClass, RosterResponse,
};
use anjuman_contracts::deck_options::{CreateDeckOptions, DeckOptions, UpdateDeckOptions};
use anjuman_contracts::decks::{
    AddDeckToClass, ClassInfo, CollaboratorResponse, CreateDeck, DeckCounts, DeckCountsResponse,
    DeckDetailResponse, DeckResponse, ShareDeck, TransferOwner, UpdateDeck,
};
use anjuman_contracts::health::Health;
use anjuman_contracts::note_types::{
    CreateTemplate, NoteTypeResponse, ReorderTemplates, Template, UpdateNoteType, UpdateTemplate,
};
use anjuman_contracts::notes::{CardSummary, CreateNote, NoteResponse, UpdateNote};
use anjuman_contracts::preferences::{UpdatePreferences, UserPreferences};
use anjuman_contracts::reviews::{
    FlagResponse, ReviewResponse, ReviewedCardState, SetFlag, SubmitReview,
};
use anjuman_contracts::shared::MessageResponse;
use anjuman_contracts::study::{StudyAdvance, StudyAdvanceBody, StudyCard, StudyCounts};
use anjuman_contracts::users::{CreateUser, SearchResult, UpdateUser, UserDetail};

/// Security requirement name referenced by the path definitions.
const JWT: &str = "bearerAuth";

/// Add the shared bearer-auth security scheme to the generated document.
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let scheme = SecurityScheme::Http(
            HttpBuilder::new()
                .scheme(HttpAuthScheme::Bearer)
                .bearer_format("JWT")
                .build(),
        );
        let components = openapi
            .components
            .get_or_insert_with(utoipa::openapi::Components::default);
        components.security_schemes.insert(JWT.to_string(), scheme);
    }
}

/// The OpenAPI document.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Anki Classroom API",
        version = "0.1.0",
        description = "Backend API for the Anki Classroom spaced repetition platform. All authenticated endpoints require a JWT in the `Authorization: Bearer <token>` header. Tokens are obtained via `POST /auth/login` and expire after 24 hours. Roles: `admin`, `teacher`, `student`."
    ),
    servers((url = "http://localhost:3000", description = "Development server")),
    modifiers(&SecurityAddon),
    paths(
        health,
        login,
        logout,
        search_users,
        list_users,
        create_user,
        get_user,
        update_user,
        delete_user,
        me,
        list_classes,
        create_class,
        get_class,
        delete_class,
        rename_class,
        archive_class,
        view_roster,
        add_member,
        remove_member,
        list_decks,
        create_deck,
        deck_counts,
        get_deck,
        delete_deck,
        rename_deck,
        duplicate_deck,
        share_deck,
        unshare_deck,
        transfer_owner,
        add_deck_to_class,
        list_deck_classes,
        remove_deck_from_class,
        deck_study,
        deck_study_advance,
        list_note_types,
        clone_note_type,
        get_note_type,
        update_note_type,
        delete_note_type,
        create_template,
        update_template,
        delete_template,
        reorder_templates,
        create_note,
        list_notes,
        get_note,
        update_note,
        delete_note,
        move_card,
        browse_cards,
        submit_review,
        set_flag,
        suspend_card,
        unsuspend_card,
        bury_card,
        unbury_card,
        reschedule_card,
        suspend_note,
        bury_note,
        unbury_note,
        unsuspend_note,
        list_deck_options,
        create_deck_options,
        get_deck_options,
        update_deck_options,
        delete_deck_options,
        my_stats,
        my_daily,
        class_analytics,
        student_detail,
        dashboard,
        get_preferences,
        update_preferences
    ),
    tags(
        (name = "health", description = "Service liveness"),
        (name = "auth", description = "Authentication"),
        (name = "users", description = "User management"),
        (name = "classes", description = "Class management"),
        (name = "decks", description = "Deck management"),
        (name = "notes", description = "Note management"),
        (name = "note-types", description = "Note types and card templates"),
        (name = "study", description = "Study flow"),
        (name = "reviews", description = "Card reviews and scheduling"),
        (name = "cards", description = "Card browser and modifications"),
        (name = "deck-options", description = "Deck options presets"),
        (name = "analytics", description = "Analytics and progress"),
        (name = "dashboard", description = "Teacher dashboard"),
        (name = "preferences", description = "User preferences")
    ),
    components(
        schemas(
            AttentionStudent,
            ClassCard,
            DifficultCard,
            StudentStatsWithEmail,
            CardSummary,
            ReviewedCardState,
            StudyCard,
            StudyCounts,
            DeckCounts,
            CardBrowserResponse,
            UserPreferences,
            UpdatePreferences
        )
    )
)]
pub struct ApiDoc;

// ---------------------------------------------------------------------------
// Auth & health
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    operation_id = "health",
    responses(
        (status = 200, description = "Server is running", body = Health)
    )
)]
fn health() {}

#[utoipa::path(
    post,
    path = "/auth/login",
    tag = "auth",
    operation_id = "login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = LoginResponse),
        (status = 401, description = "Invalid email or password", body = MessageResponse)
    )
)]
fn login() {}

#[utoipa::path(
    post,
    path = "/auth/logout",
    tag = "auth",
    operation_id = "logout",
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Logged out", body = MessageResponse),
        (status = 401, description = "Missing or invalid JWT", body = MessageResponse)
    )
)]
fn logout() {}

// ---------------------------------------------------------------------------
// Users
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/users/search",
    tag = "users",
    operation_id = "searchUsers",
    security(("bearerAuth" = [])),
    params(
        ("q" = String, Query, description = "Search term")
    ),
    responses(
        (status = 200, description = "Matching users", body = Vec<SearchResult>),
        (status = 403, description = "Students cannot search for users", body = MessageResponse)
    )
)]
fn search_users() {}

#[utoipa::path(
    get,
    path = "/users",
    tag = "users",
    operation_id = "listUsers",
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "User list", body = Vec<UserDetail>),
        (status = 403, description = "Admin role required", body = MessageResponse)
    )
)]
fn list_users() {}

#[utoipa::path(
    post,
    path = "/users",
    tag = "users",
    operation_id = "createUser",
    request_body = CreateUser,
    responses(
        (status = 200, description = "User created", body = UserResponse),
        (status = 409, description = "A user with that email already exists", body = MessageResponse)
    )
)]
fn create_user() {}

#[utoipa::path(
    get,
    path = "/users/{id}",
    tag = "users",
    operation_id = "getUser",
    security(("bearerAuth" = [])),
    params(("id" = i64, Path, description = "User id")),
    responses(
        (status = 200, description = "User details", body = UserDetail),
        (status = 403, description = "Admin role required", body = MessageResponse),
        (status = 404, description = "User not found", body = MessageResponse)
    )
)]
fn get_user() {}

#[utoipa::path(
    patch,
    path = "/users/{id}",
    tag = "users",
    operation_id = "updateUser",
    security(("bearerAuth" = [])),
    params(("id" = i64, Path, description = "User id")),
    request_body = UpdateUser,
    responses(
        (status = 200, description = "User updated", body = UserDetail),
        (status = 403, description = "Admin role required", body = MessageResponse),
        (status = 404, description = "User not found", body = MessageResponse)
    )
)]
fn update_user() {}

#[utoipa::path(
    delete,
    path = "/users/{id}",
    tag = "users",
    operation_id = "deleteUser",
    security(("bearerAuth" = [])),
    params(("id" = i64, Path, description = "User id")),
    responses(
        (status = 200, description = "User deleted", body = MessageResponse),
        (status = 403, description = "Admin role required", body = MessageResponse),
        (status = 404, description = "User not found", body = MessageResponse)
    )
)]
fn delete_user() {}

#[utoipa::path(
    get,
    path = "/me",
    tag = "users",
    operation_id = "me",
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Current user", body = UserResponse),
        (status = 401, description = "Missing or invalid JWT", body = MessageResponse)
    )
)]
fn me() {}

// ---------------------------------------------------------------------------
// Classes
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/classes",
    tag = "classes",
    operation_id = "listClasses",
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Class list", body = Vec<ClassResponse>)
    )
)]
fn list_classes() {}

#[utoipa::path(
    post,
    path = "/classes",
    tag = "classes",
    operation_id = "createClass",
    security(("bearerAuth" = [])),
    request_body = CreateClass,
    responses(
        (status = 201, description = "Class created", body = ClassResponse),
        (status = 403, description = "Only teachers and admins can manage classes", body = MessageResponse)
    )
)]
fn create_class() {}

#[utoipa::path(
    get,
    path = "/classes/{id}",
    tag = "classes",
    operation_id = "getClass",
    security(("bearerAuth" = [])),
    params(("id" = i64, Path, description = "Class id")),
    responses(
        (status = 200, description = "Class details", body = ClassResponse),
        (status = 403, description = "You are not a member of this class", body = MessageResponse),
        (status = 404, description = "Class not found", body = MessageResponse)
    )
)]
fn get_class() {}

#[utoipa::path(
    delete,
    path = "/classes/{id}",
    tag = "classes",
    operation_id = "deleteClass",
    security(("bearerAuth" = [])),
    params(("id" = i64, Path, description = "Class id")),
    responses(
        (status = 200, description = "Class deleted", body = MessageResponse),
        (status = 403, description = "Only the owner (or admin) can delete a class", body = MessageResponse)
    )
)]
fn delete_class() {}

#[utoipa::path(
    patch,
    path = "/classes/{id}/rename",
    tag = "classes",
    operation_id = "renameClass",
    security(("bearerAuth" = [])),
    params(("id" = i64, Path, description = "Class id")),
    request_body = RenameClass,
    responses(
        (status = 200, description = "Class renamed", body = ClassResponse),
        (status = 403, description = "Only teachers and admins can manage classes", body = MessageResponse)
    )
)]
fn rename_class() {}

#[utoipa::path(
    post,
    path = "/classes/{id}/archive",
    tag = "classes",
    operation_id = "archiveClass",
    security(("bearerAuth" = [])),
    params(("id" = i64, Path, description = "Class id")),
    responses(
        (status = 200, description = "Archive status toggled", body = ClassResponse),
        (status = 403, description = "Only teachers and admins can manage classes", body = MessageResponse)
    )
)]
fn archive_class() {}

#[utoipa::path(
    get,
    path = "/classes/{id}/roster",
    tag = "classes",
    operation_id = "viewRoster",
    security(("bearerAuth" = [])),
    params(("id" = i64, Path, description = "Class id")),
    responses(
        (status = 200, description = "Class with member list", body = RosterResponse),
        (status = 403, description = "You are not a member of this class", body = MessageResponse)
    )
)]
fn view_roster() {}

#[utoipa::path(
    post,
    path = "/classes/{id}/members",
    tag = "classes",
    operation_id = "addMember",
    security(("bearerAuth" = [])),
    params(("id" = i64, Path, description = "Class id")),
    request_body = AddMember,
    responses(
        (status = 201, description = "Member added", body = MemberResponse),
        (status = 400, description = "Admins cannot be added to classes", body = MessageResponse),
        (status = 404, description = "User not found in your school", body = MessageResponse)
    )
)]
fn add_member() {}

#[utoipa::path(
    delete,
    path = "/classes/{id}/members/{user_id}",
    tag = "classes",
    operation_id = "removeMember",
    security(("bearerAuth" = [])),
    params(
        ("id" = i64, Path, description = "Class id"),
        ("user_id" = i64, Path, description = "User id")
    ),
    responses(
        (status = 200, description = "Member removed", body = MessageResponse),
        (status = 404, description = "Member not found in class", body = MessageResponse)
    )
)]
fn remove_member() {}

// ---------------------------------------------------------------------------
// Decks
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/decks",
    tag = "decks",
    operation_id = "listDecks",
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Deck list", body = Vec<DeckResponse>)
    )
)]
fn list_decks() {}

#[utoipa::path(
    post,
    path = "/decks",
    tag = "decks",
    operation_id = "createDeck",
    security(("bearerAuth" = [])),
    request_body = CreateDeck,
    responses(
        (status = 201, description = "Deck created", body = DeckResponse),
        (status = 403, description = "Only teachers and admins can manage decks", body = MessageResponse)
    )
)]
fn create_deck() {}

#[utoipa::path(
    get,
    path = "/decks/counts",
    tag = "decks",
    operation_id = "deckCounts",
    security(("bearerAuth" = [])),
    params(
        ("deck_id" = Option<i64>, Query, description = "Optional: only return counts for this single deck")
    ),
    responses(
        (status = 200, description = "Deck counts", body = DeckCountsResponse)
    )
)]
fn deck_counts() {}

#[utoipa::path(
    get,
    path = "/decks/{id}",
    tag = "decks",
    operation_id = "getDeck",
    security(("bearerAuth" = [])),
    params(("id" = i64, Path, description = "Deck id")),
    responses(
        (status = 200, description = "Deck with collaborators", body = DeckDetailResponse),
        (status = 404, description = "Deck not found", body = MessageResponse)
    )
)]
fn get_deck() {}

#[utoipa::path(
    delete,
    path = "/decks/{id}",
    tag = "decks",
    operation_id = "deleteDeck",
    security(("bearerAuth" = [])),
    params(
        ("id" = i64, Path, description = "Deck id"),
        ("cascade" = Option<bool>, Query, description = "If true, also delete descendant decks recursively")
    ),
    responses(
        (status = 200, description = "Deck deleted", body = MessageResponse),
        (status = 403, description = "Only the owner (or admin) can delete a deck", body = MessageResponse)
    )
)]
fn delete_deck() {}

#[utoipa::path(
    patch,
    path = "/decks/{id}/rename",
    tag = "decks",
    operation_id = "renameDeck",
    security(("bearerAuth" = [])),
    params(("id" = i64, Path, description = "Deck id")),
    request_body = UpdateDeck,
    responses(
        (status = 200, description = "Deck renamed or moved", body = DeckResponse),
        (status = 403, description = "Only the owner, collaborator, or admin can modify this deck", body = MessageResponse)
    )
)]
fn rename_deck() {}

#[utoipa::path(
    post,
    path = "/decks/{id}/duplicate",
    tag = "decks",
    operation_id = "duplicateDeck",
    security(("bearerAuth" = [])),
    params(("id" = i64, Path, description = "Deck id")),
    responses(
        (status = 201, description = "Deck duplicated", body = DeckResponse),
        (status = 403, description = "Only teachers and admins can duplicate decks", body = MessageResponse)
    )
)]
fn duplicate_deck() {}

#[utoipa::path(
    post,
    path = "/decks/{id}/share",
    tag = "decks",
    operation_id = "shareDeck",
    security(("bearerAuth" = [])),
    params(("id" = i64, Path, description = "Deck id")),
    request_body = ShareDeck,
    responses(
        (status = 200, description = "Deck shared", body = CollaboratorResponse),
        (status = 400, description = "Cannot share with a student, admin, or the deck owner", body = MessageResponse)
    )
)]
fn share_deck() {}

#[utoipa::path(
    delete,
    path = "/decks/{id}/share/{user_id}",
    tag = "decks",
    operation_id = "unshareDeck",
    security(("bearerAuth" = [])),
    params(
        ("id" = i64, Path, description = "Deck id"),
        ("user_id" = i64, Path, description = "User id")
    ),
    responses(
        (status = 200, description = "Collaborator removed", body = MessageResponse)
    )
)]
fn unshare_deck() {}

#[utoipa::path(
    patch,
    path = "/decks/{id}/owner",
    tag = "decks",
    operation_id = "transferOwner",
    security(("bearerAuth" = [])),
    params(("id" = i64, Path, description = "Deck id")),
    request_body = TransferOwner,
    responses(
        (status = 200, description = "Ownership transferred", body = DeckResponse),
        (status = 400, description = "Target is not a teacher", body = MessageResponse)
    )
)]
fn transfer_owner() {}

#[utoipa::path(
    post,
    path = "/decks/{id}/classes",
    tag = "decks",
    operation_id = "addDeckToClass",
    security(("bearerAuth" = [])),
    params(("id" = i64, Path, description = "Deck id")),
    request_body = AddDeckToClass,
    responses(
        (status = 200, description = "Deck added to class", body = MessageResponse),
        (status = 400, description = "Bad request", body = MessageResponse)
    )
)]
fn add_deck_to_class() {}

#[utoipa::path(
    get,
    path = "/decks/{id}/classes",
    tag = "decks",
    operation_id = "listDeckClasses",
    security(("bearerAuth" = [])),
    params(("id" = i64, Path, description = "Deck id")),
    responses(
        (status = 200, description = "List of classes", body = Vec<ClassInfo>)
    )
)]
fn list_deck_classes() {}

#[utoipa::path(
    delete,
    path = "/decks/{id}/classes/{class_id}",
    tag = "decks",
    operation_id = "removeDeckFromClass",
    security(("bearerAuth" = [])),
    params(
        ("id" = i64, Path, description = "Deck id"),
        ("class_id" = i64, Path, description = "Class id")
    ),
    responses(
        (status = 200, description = "Deck removed from class", body = MessageResponse)
    )
)]
fn remove_deck_from_class() {}

// ---------------------------------------------------------------------------
// Study
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/decks/{id}/study",
    tag = "study",
    operation_id = "deckStudy",
    security(("bearerAuth" = [])),
    params(("id" = i64, Path, description = "Deck id")),
    responses(
        (status = 200, description = "Next due card and counts", body = StudyAdvance),
        (status = 403, description = "Deck is not accessible for study", body = MessageResponse)
    )
)]
fn deck_study() {}

#[utoipa::path(
    post,
    path = "/decks/{id}/study",
    tag = "study",
    operation_id = "deckStudyAdvance",
    security(("bearerAuth" = [])),
    params(("id" = i64, Path, description = "Deck id")),
    request_body = StudyAdvanceBody,
    responses(
        (status = 200, description = "Next due card, reviewed card state, and counts", body = StudyAdvance),
        (status = 400, description = "Invalid rating or card does not belong to this deck", body = MessageResponse)
    )
)]
fn deck_study_advance() {}

// ---------------------------------------------------------------------------
// Note types
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/note-types",
    tag = "note-types",
    operation_id = "listNoteTypes",
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "List of note types", body = Vec<NoteTypeResponse>)
    )
)]
fn list_note_types() {}

#[utoipa::path(
    post,
    path = "/note-types/{id}/clone",
    tag = "note-types",
    operation_id = "cloneNoteType",
    security(("bearerAuth" = [])),
    params(("id" = i64, Path, description = "Note type id")),
    responses(
        (status = 201, description = "Note type cloned", body = NoteTypeResponse),
        (status = 403, description = "Only teachers and admins can manage note types", body = MessageResponse)
    )
)]
fn clone_note_type() {}

#[utoipa::path(
    get,
    path = "/note-types/{id}",
    tag = "note-types",
    operation_id = "getNoteType",
    security(("bearerAuth" = [])),
    params(("id" = i64, Path, description = "Note type id")),
    responses(
        (status = 200, description = "Note type details", body = NoteTypeResponse),
        (status = 404, description = "Note type not found", body = MessageResponse)
    )
)]
fn get_note_type() {}

#[utoipa::path(
    patch,
    path = "/note-types/{id}",
    tag = "note-types",
    operation_id = "updateNoteType",
    security(("bearerAuth" = [])),
    params(("id" = i64, Path, description = "Note type id")),
    request_body = UpdateNoteType,
    responses(
        (status = 200, description = "Note type updated", body = NoteTypeResponse),
        (status = 403, description = "Only teachers and admins can manage note types", body = MessageResponse)
    )
)]
fn update_note_type() {}

#[utoipa::path(
    delete,
    path = "/note-types/{id}",
    tag = "note-types",
    operation_id = "deleteNoteType",
    security(("bearerAuth" = [])),
    params(("id" = i64, Path, description = "Note type id")),
    responses(
        (status = 200, description = "Note type deleted", body = MessageResponse),
        (status = 403, description = "Only teachers and admins can manage note types", body = MessageResponse)
    )
)]
fn delete_note_type() {}

#[utoipa::path(
    post,
    path = "/note-types/{id}/templates",
    tag = "note-types",
    operation_id = "createTemplate",
    security(("bearerAuth" = [])),
    params(("id" = i64, Path, description = "Note type id")),
    request_body = CreateTemplate,
    responses(
        (status = 201, description = "Template appended", body = Template),
        (status = 403, description = "Only teachers and admins can manage note types", body = MessageResponse)
    )
)]
fn create_template() {}

#[utoipa::path(
    patch,
    path = "/note-types/{id}/templates/{template_id}",
    tag = "note-types",
    operation_id = "updateTemplate",
    security(("bearerAuth" = [])),
    params(
        ("id" = i64, Path, description = "Note type id"),
        ("template_id" = i64, Path, description = "Template id")
    ),
    request_body = UpdateTemplate,
    responses(
        (status = 200, description = "Template updated", body = Template),
        (status = 404, description = "Template not found", body = MessageResponse)
    )
)]
fn update_template() {}

#[utoipa::path(
    delete,
    path = "/note-types/{id}/templates/{template_id}",
    tag = "note-types",
    operation_id = "deleteTemplate",
    security(("bearerAuth" = [])),
    params(
        ("id" = i64, Path, description = "Note type id"),
        ("template_id" = i64, Path, description = "Template id")
    ),
    responses(
        (status = 200, description = "Template deleted", body = MessageResponse),
        (status = 404, description = "Template not found", body = MessageResponse)
    )
)]
fn delete_template() {}

#[utoipa::path(
    patch,
    path = "/note-types/{id}/templates/order",
    tag = "note-types",
    operation_id = "reorderTemplates",
    security(("bearerAuth" = [])),
    params(("id" = i64, Path, description = "Note type id")),
    request_body = ReorderTemplates,
    responses(
        (status = 200, description = "Templates reordered", body = Vec<Template>)
    )
)]
fn reorder_templates() {}

// ---------------------------------------------------------------------------
// Notes
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/notes",
    tag = "notes",
    operation_id = "createNote",
    security(("bearerAuth" = [])),
    request_body = CreateNote,
    responses(
        (status = 201, description = "Note created with auto-generated cards", body = NoteResponse),
        (status = 400, description = "Unknown note type or missing fields", body = MessageResponse)
    )
)]
fn create_note() {}

#[utoipa::path(
    get,
    path = "/notes",
    tag = "notes",
    operation_id = "listNotes",
    security(("bearerAuth" = [])),
    params(
        ("deck_id" = Option<i64>, Query, description = "Optional deck filter: only notes with at least one card in this deck")
    ),
    responses(
        (status = 200, description = "Notes with cards", body = Vec<NoteResponse>)
    )
)]
fn list_notes() {}

#[utoipa::path(
    get,
    path = "/notes/{note_id}",
    tag = "notes",
    operation_id = "getNote",
    security(("bearerAuth" = [])),
    params(("note_id" = i64, Path, description = "Note id")),
    responses(
        (status = 200, description = "Note with cards", body = NoteResponse),
        (status = 404, description = "Note not found", body = MessageResponse)
    )
)]
fn get_note() {}

#[utoipa::path(
    patch,
    path = "/notes/{note_id}",
    tag = "notes",
    operation_id = "updateNote",
    security(("bearerAuth" = [])),
    params(("note_id" = i64, Path, description = "Note id")),
    request_body = UpdateNote,
    responses(
        (status = 200, description = "Note updated, cards re-rendered", body = NoteResponse),
        (status = 400, description = "Unknown note type or missing fields", body = MessageResponse)
    )
)]
fn update_note() {}

#[utoipa::path(
    delete,
    path = "/notes/{note_id}",
    tag = "notes",
    operation_id = "deleteNote",
    security(("bearerAuth" = [])),
    params(("note_id" = i64, Path, description = "Note id")),
    responses(
        (status = 200, description = "Note deleted", body = MessageResponse)
    )
)]
fn delete_note() {}

// ---------------------------------------------------------------------------
// Cards
// ---------------------------------------------------------------------------

#[utoipa::path(
    patch,
    path = "/cards/{card_id}/deck",
    tag = "cards",
    operation_id = "moveCard",
    security(("bearerAuth" = [])),
    params(("card_id" = i64, Path, description = "Card id")),
    request_body = MoveCardBody,
    responses(
        (status = 200, description = "Card moved", body = MoveCardResponse)
    )
)]
fn move_card() {}

#[utoipa::path(
    get,
    path = "/cards",
    tag = "cards",
    operation_id = "browseCards",
    security(("bearerAuth" = [])),
    params(
        ("deck_id" = Option<String>, Query, description = "Comma-separated deck IDs (e.g. '1,2,3')"),
        ("note_type_id" = Option<String>, Query, description = "Comma-separated note type IDs (e.g. '1,2')"),
        ("q" = Option<String>, Query, description = "Search in card content (matches fields JSON)"),
        ("sort" = Option<String>, Query, description = "Sort order: created_at, due_at, deck, question (default created_at)"),
        ("state" = Option<String>, Query, description = "Comma-separated card states: new, learning, review, relearning"),
        ("flag" = Option<String>, Query, description = "Comma-separated flag values (0-7)"),
        ("page" = Option<i64>, Query, description = "Page number (default 1)"),
        ("per_page" = Option<i64>, Query, description = "Results per page (default 50, max 100)")
    ),
    responses(
        (status = 200, description = "Paginated card list", body = CardBrowserPage)
    )
)]
fn browse_cards() {}

// ---------------------------------------------------------------------------
// Reviews & scheduling
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/reviews",
    tag = "reviews",
    operation_id = "submitReview",
    security(("bearerAuth" = [])),
    request_body = SubmitReview,
    responses(
        (status = 200, description = "Review processed, new schedule returned", body = ReviewResponse),
        (status = 400, description = "Rating must be 1-4, or card state not found", body = MessageResponse)
    )
)]
fn submit_review() {}

#[utoipa::path(
    patch,
    path = "/cards/{card_id}/flag",
    tag = "reviews",
    operation_id = "setFlag",
    security(("bearerAuth" = [])),
    params(("card_id" = i64, Path, description = "Card id")),
    request_body = SetFlag,
    responses(
        (status = 200, description = "Flag set", body = FlagResponse),
        (status = 400, description = "Flag must be between 0 and 7", body = MessageResponse)
    )
)]
fn set_flag() {}

#[utoipa::path(
    post,
    path = "/cards/{card_id}/suspend",
    tag = "cards",
    operation_id = "suspendCard",
    security(("bearerAuth" = [])),
    params(("card_id" = i64, Path, description = "Card id")),
    responses(
        (status = 200, description = "Card suspended", body = CardModResponse)
    )
)]
fn suspend_card() {}

#[utoipa::path(
    post,
    path = "/cards/{card_id}/unsuspend",
    tag = "cards",
    operation_id = "unsuspendCard",
    security(("bearerAuth" = [])),
    params(("card_id" = i64, Path, description = "Card id")),
    responses(
        (status = 200, description = "Card unsuspended", body = CardModResponse)
    )
)]
fn unsuspend_card() {}

#[utoipa::path(
    post,
    path = "/cards/{card_id}/bury",
    tag = "cards",
    operation_id = "buryCard",
    security(("bearerAuth" = [])),
    params(("card_id" = i64, Path, description = "Card id")),
    responses(
        (status = 200, description = "Card buried until the next day-start", body = CardModResponse)
    )
)]
fn bury_card() {}

#[utoipa::path(
    post,
    path = "/cards/{card_id}/unbury",
    tag = "cards",
    operation_id = "unburyCard",
    security(("bearerAuth" = [])),
    params(
        ("card_id" = i64, Path, description = "Card id"),
        ("reason" = Option<String>, Query, description = "Optional bury reason to clear")
    ),
    responses(
        (status = 200, description = "Card unburied", body = CardModResponse)
    )
)]
fn unbury_card() {}

#[utoipa::path(
    patch,
    path = "/cards/{card_id}/reschedule",
    tag = "cards",
    operation_id = "rescheduleCard",
    security(("bearerAuth" = [])),
    params(("card_id" = i64, Path, description = "Card id")),
    request_body = RescheduleBody,
    responses(
        (status = 200, description = "Card rescheduled", body = CardModResponse)
    )
)]
fn reschedule_card() {}

#[utoipa::path(
    post,
    path = "/notes/{note_id}/suspend",
    tag = "cards",
    operation_id = "suspendNote",
    security(("bearerAuth" = [])),
    params(("note_id" = i64, Path, description = "Note id")),
    responses(
        (status = 200, description = "Note cards suspended", body = NoteModResponse)
    )
)]
fn suspend_note() {}

#[utoipa::path(
    post,
    path = "/notes/{note_id}/bury",
    tag = "cards",
    operation_id = "buryNote",
    security(("bearerAuth" = [])),
    params(("note_id" = i64, Path, description = "Note id")),
    responses(
        (status = 200, description = "Note cards buried until the next day-start", body = NoteModResponse)
    )
)]
fn bury_note() {}

#[utoipa::path(
    post,
    path = "/notes/{note_id}/unbury",
    tag = "cards",
    operation_id = "unburyNote",
    security(("bearerAuth" = [])),
    params(
        ("note_id" = i64, Path, description = "Note id"),
        ("reason" = Option<String>, Query, description = "Optional bury reason to clear")
    ),
    responses(
        (status = 200, description = "Note cards unburied", body = NoteModResponse)
    )
)]
fn unbury_note() {}

#[utoipa::path(
    post,
    path = "/notes/{note_id}/unsuspend",
    tag = "cards",
    operation_id = "unsuspendNote",
    security(("bearerAuth" = [])),
    params(("note_id" = i64, Path, description = "Note id")),
    responses(
        (status = 200, description = "Note cards unsuspended", body = NoteModResponse)
    )
)]
fn unsuspend_note() {}

// ---------------------------------------------------------------------------
// Deck options
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/deck-options",
    tag = "deck-options",
    operation_id = "listDeckOptions",
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "List of presets", body = Vec<DeckOptions>)
    )
)]
fn list_deck_options() {}

#[utoipa::path(
    post,
    path = "/deck-options",
    tag = "deck-options",
    operation_id = "createDeckOptions",
    security(("bearerAuth" = [])),
    request_body = CreateDeckOptions,
    responses(
        (status = 201, description = "Preset created", body = DeckOptions),
        (status = 403, description = "Only teachers and admins can manage deck options", body = MessageResponse)
    )
)]
fn create_deck_options() {}

#[utoipa::path(
    get,
    path = "/deck-options/{id}",
    tag = "deck-options",
    operation_id = "getDeckOptions",
    security(("bearerAuth" = [])),
    params(("id" = i64, Path, description = "Preset id")),
    responses(
        (status = 200, description = "Preset details", body = DeckOptions),
        (status = 404, description = "Deck options not found", body = MessageResponse)
    )
)]
fn get_deck_options() {}

#[utoipa::path(
    patch,
    path = "/deck-options/{id}",
    tag = "deck-options",
    operation_id = "updateDeckOptions",
    security(("bearerAuth" = [])),
    params(("id" = i64, Path, description = "Preset id")),
    request_body = UpdateDeckOptions,
    responses(
        (status = 200, description = "Preset updated", body = DeckOptions),
        (status = 403, description = "Only teachers and admins can manage deck options", body = MessageResponse)
    )
)]
fn update_deck_options() {}

#[utoipa::path(
    delete,
    path = "/deck-options/{id}",
    tag = "deck-options",
    operation_id = "deleteDeckOptions",
    security(("bearerAuth" = [])),
    params(("id" = i64, Path, description = "Preset id")),
    responses(
        (status = 200, description = "Preset deleted", body = MessageResponse),
        (status = 403, description = "Only teachers and admins can manage deck options", body = MessageResponse)
    )
)]
fn delete_deck_options() {}

// ---------------------------------------------------------------------------
// Analytics & dashboard
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/analytics/me",
    tag = "analytics",
    operation_id = "myStats",
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Student's stats", body = StudentStats)
    )
)]
fn my_stats() {}

#[utoipa::path(
    get,
    path = "/analytics/me/daily",
    tag = "analytics",
    operation_id = "myDaily",
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Daily review data", body = Vec<DailyPoint>)
    )
)]
fn my_daily() {}

#[utoipa::path(
    get,
    path = "/analytics/classes/{id}",
    tag = "analytics",
    operation_id = "classAnalytics",
    security(("bearerAuth" = [])),
    params(("id" = i64, Path, description = "Class id")),
    responses(
        (status = 200, description = "Class analytics", body = ClassAnalytics),
        (status = 403, description = "Only class members (teacher or admin) can view analytics", body = MessageResponse)
    )
)]
fn class_analytics() {}

#[utoipa::path(
    get,
    path = "/analytics/classes/{id}/students/{student_id}",
    tag = "analytics",
    operation_id = "studentDetail",
    security(("bearerAuth" = [])),
    params(
        ("id" = i64, Path, description = "Class id"),
        ("student_id" = i64, Path, description = "Student user id")
    ),
    responses(
        (status = 200, description = "Student detail with daily data", body = StudentDetail),
        (status = 403, description = "Only class members (teacher or admin) can view analytics", body = MessageResponse)
    )
)]
fn student_detail() {}

#[utoipa::path(
    get,
    path = "/dashboard",
    tag = "dashboard",
    operation_id = "dashboard",
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Dashboard data", body = DashboardResponse),
        (status = 403, description = "Only teachers and admins can view the dashboard", body = MessageResponse)
    )
)]
fn dashboard() {}

// ---------------------------------------------------------------------------
// Preferences
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/preferences",
    tag = "preferences",
    operation_id = "getPreferences",
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Current user's preferences", body = UserPreferences),
        (status = 401, description = "Missing or invalid JWT", body = MessageResponse)
    )
)]
fn get_preferences() {}

#[utoipa::path(
    patch,
    path = "/preferences",
    tag = "preferences",
    operation_id = "updatePreferences",
    security(("bearerAuth" = [])),
    request_body = UpdatePreferences,
    responses(
        (status = 200, description = "Updated preferences", body = UserPreferences),
        (status = 400, description = "Value out of range", body = MessageResponse),
        (status = 401, description = "Missing or invalid JWT", body = MessageResponse)
    )
)]
fn update_preferences() {}

#[cfg(test)]
mod tests {
    use super::ApiDoc;
    use utoipa::OpenApi;

    /// Serialize the generated OpenAPI document to JSON (useful for
    /// inspecting/diffing the spec). Writes to a file under
    /// `std::env::temp_dir()` when the `DUMP_OPENAPI` env var is set.
    #[test]
    fn generate_openapi_json() {
        let doc = ApiDoc::openapi();
        let json = doc.to_pretty_json().expect("serialize OpenAPI");

        eprintln!("paths: {}", doc.paths.paths.len());
        eprintln!(
            "schemas: {}",
            doc.components.as_ref().map(|c| c.schemas.len()).unwrap_or(0)
        );

        if std::env::var("DUMP_OPENAPI").is_ok() {
            let path = std::env::temp_dir().join("generated-openapi.json");
            std::fs::write(&path, json).expect("write openapi json");
            eprintln!("wrote {}", path.display());
        }
    }
}
