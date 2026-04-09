/** API shapes aligned with backend JSON (snake_case). */

export interface RequirementStatus {
  id: number;
  title: string;
  description: string;
  tag: string;
  project_id: number;
  is_system: boolean;
  tag_color: string | null;
}

export interface Requirement {
  id: number;
  current_version_id: number | null;
  title: string;
  description: string;
  status_id: number;
  author_id: number;
  reviewer_id: number;
  reference_code: string;
  category_id: number;
  parent_id: number | null;
  creation_date: string;
  update_date: string;
  deadline_date: string | null;
  applicability_id: number;
  justification: string | null;
  project_id: number;
  approval_state: string;
  approved_by: number | null;
  approved_at: string | null;
  custom_fields?: Array<{
    field_id: number;
    label: string;
    value: string | null;
  }>;
  /** Present on `GET /api/projects/:id/requirements` list responses. */
  verification_method_ids?: number[];
  /** All upstream parent requirement ids (version links); list API only. `parent_id` is the first. */
  parent_requirement_ids?: number[];
}

export interface MatrixLink {
  req_id: number;
  verification_id: number;
  creation_date: string;
  project_id: number;
  suspect: boolean;
  suspect_at: string | null;
  suspect_reason: string | null;
  cleared_by: number | null;
  cleared_at: string | null;
  triggering_version_id: number | null;
  triggering_user_id: number | null;
}

/** GET/PUT `/api/projects/:projectId/verifications/:verificationId/matrix` */
export interface VerificationMatrixPayload {
  verification_id: number;
  requirement_ids: number[];
}

/** PUT body: replace all requirement↔verification matrix rows for that verification. */
export type VerificationMatrixPutBody = {
  requirement_ids: number[];
};

export interface Verification {
  id: number;
  name: string;
  reference_code: string;
  description: string;
  source: string;
  status_id: number;
  parent_id: number | null;
  project_id: number;
  verification_method_id: number | null;
  author_id: number;
  reviewer_id: number;
  status_set_by?: number | null;
  status_set_at?: string | null;
}

export interface VerificationStatus {
  id: number;
  title: string;
  description: string;
  tag: string;
  project_id: number;
  is_system: boolean;
  tag_color: string | null;
}

export interface VerificationMethod {
  id: number;
  title: string;
  description: string;
  tag: string;
  project_id: number;
}

/** POST `/api/projects/:id/requirements` */
export type RequirementCreateBody = {
  title: string;
  description: string;
  author_id: number;
  category_id: number;
  status_id: number;
  reference_code: string;
  reviewer_id: number;
  applicability_id: number;
  justification?: string | null;
  project_id: number;
  verification_method_ids: number[];
  custom_fields?: Array<{ field_id: number; value: string | null }>;
  parent_links?: Array<{
    target_version_id: number;
    link_type: string;
    rationale?: string | null;
  }>;
};

/** POST `/api/verifications` */
export type NewVerificationBody = {
  reference_code: string;
  name: string;
  description: string;
  source: string;
  status_id: number;
  parent_id: number | null;
  project_id: number;
  verification_method_id: number | null;
  author_id: number;
  reviewer_id: number;
};

/** Normalized for UI; `id` / `slug` mirror Rocket `Project` fields. */
export interface DashboardProject {
  id: number;
  name: string;
  slug: string;
  project_base_path: string;
  group_id: number | null;
  group_name: string | null;
  group_slug: string | null;
  [key: string]: unknown;
}

/**
 * Wire format from `GET /api/dashboard` (`decorate_projects_for_listing`):
 * uses `project_id` / `project_slug`, not `id` / `slug`.
 */
export interface DashboardProjectWire {
  project_id: number;
  project_slug: string;
  project_base_path: string;
  name: string;
  group_id: number | null;
  group_name: string | null;
  group_slug: string | null;
  [key: string]: unknown;
}

export interface DashboardPayloadWire {
  user: unknown;
  projects: DashboardProjectWire[];
  projects_count: number;
  selected_project_id: number | null;
  selected_project_slug: string | null;
  csrf_token: string;
}

export interface DashboardPayload {
  user: unknown;
  projects: DashboardProject[];
  projects_count: number;
  selected_project_id: number | null;
  selected_project_slug: string | null;
  csrf_token: string;
}

/** `GET /api/projects/:pid/requirements/:id` — requirement fields flattened + trace_summary. */
export interface RequirementVersionLink {
  id: number;
  source_version_id: number;
  target_version_id: number;
  link_type: string;
  rationale: string | null;
  project_id: number;
  created_at: string;
  metadata: unknown | null;
}

export interface TraceSummary {
  parent_links: RequirementVersionLink[];
  child_ids: number[];
  linked_test_ids: number[];
}

export type RequirementDetailPayload = Requirement & {
  trace_summary: TraceSummary;
};

export interface RequirementVersion {
  id: number;
  requirement_id: number;
  title: string;
  description: string;
  status_id: number;
  author_id: number;
  reviewer_id: number;
  category_id: number;
  applicability_id: number;
  justification: string | null;
  deadline_date: string | null;
  created_at: string;
  approval_state: string;
  approved_by: number | null;
  approved_at: string | null;
}

export interface Category {
  id: number;
  title: string;
  description: string;
  tag: string;
  project_id: number;
}

export interface Applicability {
  id: number;
  title: string;
  description: string;
  tag: string;
  project_id: number;
}

export interface ProjectMember {
  user_id: number;
  role: number;
  role_label: string;
  username: string;
  name: string;
}

export interface CoverageReport {
  requirements_without_tests: number[];
  tests_without_requirements: number[];
  suspect_links: Array<{ req_id: number; verification_id: number }>;
}

export interface EffectivePermissions {
  view_requirements: boolean;
  edit_requirements: boolean;
  approve_versions: boolean;
  /** May change requirement/verification status and version approval (project reviewer pool). */
  is_project_reviewer: boolean;
  manage_custom_fields: boolean;
  manage_project_members: boolean;
}

/** GET/PUT `/api/projects/:id/reviewers` */
export interface ProjectReviewersResponse {
  user_ids: number[];
}

export interface CustomFieldDefinition {
  id: number;
  project_id: number;
  label: string;
  field_type: string;
  enum_values: unknown;
  sort_order: number;
  created_at: string;
}

/** GET/POST `/api/requirements/:id/comments` */
export interface RequirementCommentItem {
  id: number;
  requirement_id: number;
  requirement_version_id: number | null;
  author_id: number;
  author_name: string;
  body: string;
  created_at: string;
}

/** GET `/api/projects/:pid/requirements/:id/activity` and `.../verifications/:id/activity` */
export interface EntityActivityChange {
  field: string;
  old_value: string;
  new_value: string;
}

export interface EntityActivityItem {
  log_id: number;
  user_id: number;
  username: string;
  action_type: string;
  summary: string;
  description: string | null;
  created_at: string;
  changes: EntityActivityChange[];
}

export interface Baseline {
  id: number;
  project_id: number;
  name: string;
  description: string | null;
  created_at: string;
  created_by: number;
}

export interface BaselineTraceabilityRow {
  baseline_id: number;
  requirement_id: number;
  verification_id: number;
  suspect: boolean;
  suspect_at: string | null;
  suspect_reason: string | null;
}

export interface BaselineVerificationSnapshot {
  baseline_id: number;
  verification_id: number;
  name: string;
  reference_code: string;
  description: string;
  source: string;
  status_id: number;
  parent_id: number | null;
  project_id: number;
  verification_method_id: number | null;
  author_id?: number;
  reviewer_id?: number;
}

export interface User {
  id: number;
  username: string;
  name: string;
  email: string;
  creation_date: string;
  last_login: string;
  is_admin: boolean;
}

/** Single custom field value in PATCH body (matches backend `CustomFieldValueInput`). */
export type CustomFieldPatchItem = { field_id: number; value: string | null };

/** Body for `PATCH /api/projects/:pid/requirements/:id` (all optional). */
export type RequirementPatchBody = {
  title?: string;
  description?: string;
  status_id?: number;
  author_id?: number;
  reviewer_id?: number;
  category_id?: number;
  applicability_id?: number;
  verification_method_ids?: number[];
  custom_fields?: CustomFieldPatchItem[];
};

/** POST/PUT `/api/categories`, `/api/applicability` (id null on create). */
export type TaggedMetadataBody = {
  id?: number | null;
  title: string;
  description: string;
  tag: string;
  project_id: number;
};

/** POST `/api/status`, PUT `/api/status/:id` */
export type RequirementStatusWriteBody = {
  id?: number | null;
  title: string;
  description: string;
  tag: string;
  project_id: number;
  is_system?: boolean;
  tag_color?: string | null;
};

/** POST `/api/verification-status`, PUT `/api/verification-status/:id` */
export type VerificationStatusWriteBody = {
  id?: number | null;
  title: string;
  description: string;
  tag: string;
  project_id: number;
  is_system?: boolean;
  tag_color?: string | null;
};

/** POST/PUT `/api/projects/:pid/custom_fields` */
export type CustomFieldWriteBody = {
  label: string;
  field_type: string;
  enum_values?: string[] | null;
  sort_order?: number | null;
};

/** POST/PUT `/api/projects/:pid/verification-methods` */
export type VerificationMethodWriteBody = {
  id?: number | null;
  title: string;
  description: string;
  tag: string;
  project_id: number;
};

/* ——— Groups ——— */

export interface GroupResponse {
  id: number;
  name: string;
  slug: string;
  description: string | null;
  owner_id: number | null;
  created_at: string;
  updated_at: string;
}

export interface GroupMemberResponse {
  user_id: number;
  role: number;
  role_label: string;
}

/* ——— Notifications ——— */

export interface Notification {
  id: number;
  user_id: number;
  project_id: number | null;
  notification_type: string;
  title: string;
  body: string | null;
  entity_type: string | null;
  entity_id: number | null;
  actor_id: number | null;
  read: boolean;
  emailed: boolean;
  created_at: string;
}

export interface NotificationPreference {
  id: number;
  user_id: number;
  project_id: number;
  notify_in_app: boolean;
  notify_email: boolean;
}

export interface ProjectFromPath {
  id: number;
  name: string;
  description: string | null;
  slug: string;
  route_slug: string;
}

export interface Project {
  id: number;
  name: string;
  slug: string;
  description: string | null;
  owner_id: number | null;
  group_id: number | null;
  status: string;
  creation_date: string | null;
  update_date: string | null;
}
