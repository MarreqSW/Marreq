// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use super::{map_db_error, DieselRepo};
use crate::models::entities::*;
use crate::models::forms::*;
use crate::repository::errors::RepoError;
use crate::repository::CustomFieldRepository;
use crate::schema;
use diesel::prelude::*;
use diesel::JoinOnDsl;

impl CustomFieldRepository for DieselRepo {
    fn list_custom_field_definitions_by_project(
        &self,
        project_id: i32,
    ) -> Result<Vec<CustomFieldDefinition>, RepoError> {
        use schema::custom_field_definitions::dsl;
        let mut conn = self.get_conn()?;
        dsl::custom_field_definitions
            .filter(dsl::project_id.eq(project_id))
            .order((dsl::sort_order, dsl::id))
            .load(conn.as_mut())
            .map_err(RepoError::from)
    }

    fn get_custom_field_definition_by_id(
        &self,
        id: i32,
    ) -> Result<CustomFieldDefinition, RepoError> {
        use schema::custom_field_definitions::dsl;
        let mut conn = self.get_conn()?;
        dsl::custom_field_definitions
            .filter(dsl::id.eq(id))
            .get_result(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })
    }

    fn create_custom_field_definition(
        &mut self,
        project_id: i32,
        payload: &CustomFieldDefinitionPayload,
    ) -> Result<i32, RepoError> {
        let enum_values_json = payload
            .enum_values
            .as_ref()
            .map(|v| serde_json::to_value(v).map_err(|e| RepoError::BadInput(e.to_string())))
            .transpose()?;
        let row = NewCustomFieldDefinitionRow {
            project_id,
            label: payload.label.trim().to_string(),
            field_type: payload.field_type.trim().to_lowercase(),
            enum_values: enum_values_json,
            sort_order: payload.sort_order.unwrap_or(0),
        };
        validate_custom_field_payload(&row.field_type, row.enum_values.as_ref())?;
        use schema::custom_field_definitions::dsl;
        let mut conn = self.get_conn()?;
        diesel::insert_into(dsl::custom_field_definitions)
            .values(&row)
            .returning(dsl::id)
            .get_result(conn.as_mut())
            .map_err(RepoError::from)
    }

    fn update_custom_field_definition(
        &mut self,
        id: i32,
        payload: &CustomFieldDefinitionPayload,
    ) -> Result<(), RepoError> {
        let enum_values_json = payload
            .enum_values
            .as_ref()
            .map(|v| serde_json::to_value(v).map_err(|e| RepoError::BadInput(e.to_string())))
            .transpose()?;
        let field_type = payload.field_type.trim().to_lowercase();
        validate_custom_field_payload(&field_type, enum_values_json.as_ref())?;
        use schema::custom_field_definitions::dsl;
        let mut conn = self.get_conn()?;
        let affected = diesel::update(dsl::custom_field_definitions.filter(dsl::id.eq(id)))
            .set((
                dsl::label.eq(payload.label.trim()),
                dsl::field_type.eq(&field_type),
                dsl::enum_values.eq(enum_values_json),
                dsl::sort_order.eq(payload.sort_order.unwrap_or(0)),
            ))
            .execute(conn.as_mut())?;
        if affected == 0 {
            return Err(RepoError::NotFound);
        }
        Ok(())
    }

    fn count_requirement_versions_using_field(&self, field_id: i32) -> Result<i64, RepoError> {
        use schema::custom_field_values::dsl;
        let mut conn = self.get_conn()?;
        dsl::custom_field_values
            .filter(dsl::custom_field_definition_id.eq(field_id))
            .count()
            .get_result(conn.as_mut())
            .map_err(RepoError::from)
    }

    fn delete_custom_field_definition(&mut self, id: i32) -> Result<(), RepoError> {
        use schema::custom_field_definitions::dsl;
        let mut conn = self.get_conn()?;
        let affected = diesel::delete(dsl::custom_field_definitions.filter(dsl::id.eq(id)))
            .execute(conn.as_mut())?;
        if affected == 0 {
            return Err(RepoError::NotFound);
        }
        Ok(())
    }

    fn get_custom_field_values_for_version(
        &self,
        version_id: i32,
    ) -> Result<Vec<CustomFieldValueDisplay>, RepoError> {
        use schema::custom_field_definitions;
        use schema::custom_field_values;
        let mut conn = self.get_conn()?;
        let rows: Vec<(CustomFieldValue, CustomFieldDefinition)> = custom_field_values::table
            .inner_join(custom_field_definitions::table.on(
                custom_field_values::custom_field_definition_id.eq(custom_field_definitions::id),
            ))
            .filter(custom_field_values::requirement_version_id.eq(version_id))
            .select((
                custom_field_values::all_columns,
                custom_field_definitions::all_columns,
            ))
            .load(conn.as_mut())
            .map_err(RepoError::from)?;
        Ok(rows
            .into_iter()
            .map(|(v, d)| CustomFieldValueDisplay {
                field_id: d.id,
                label: d.label,
                value: v.value,
            })
            .collect())
    }

    fn set_custom_field_values_for_version(
        &mut self,
        version_id: i32,
        values: &[(i32, Option<String>)],
    ) -> Result<(), RepoError> {
        use schema::custom_field_values::dsl;
        let mut conn = self.get_conn()?;
        diesel::delete(dsl::custom_field_values.filter(dsl::requirement_version_id.eq(version_id)))
            .execute(conn.as_mut())?;
        for &(field_id, ref value) in values {
            if field_id <= 0 {
                continue;
            }
            diesel::insert_into(dsl::custom_field_values)
                .values((
                    dsl::requirement_version_id.eq(version_id),
                    dsl::custom_field_definition_id.eq(field_id),
                    dsl::value.eq(value.as_deref()),
                ))
                .execute(conn.as_mut())
                .map_err(map_db_error)?;
        }
        Ok(())
    }
}

fn validate_custom_field_payload(
    field_type: &str,
    enum_values: Option<&serde_json::Value>,
) -> Result<(), RepoError> {
    const VALID_TYPES: &[&str] = &["text", "enum", "boolean", "number"];
    if !VALID_TYPES.contains(&field_type) {
        return Err(RepoError::BadInput(format!(
            "field_type must be one of: {}",
            VALID_TYPES.join(", ")
        )));
    }
    if field_type == "enum" {
        let arr = enum_values.and_then(|v| v.as_array()).ok_or_else(|| {
            RepoError::BadInput("enum field_type requires enum_values array".into())
        })?;
        if arr.is_empty() {
            return Err(RepoError::BadInput(
                "enum field_type requires at least one enum value".into(),
            ));
        }
    }
    Ok(())
}
