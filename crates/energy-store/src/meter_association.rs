// SPDX-License-Identifier: MIT
//! Atomic, idempotent physical-meter resource associations.

use sqlx::MySqlPool;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssociationOutcome {
    Associated,
    AlreadyAssociated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssociationError {
    InvalidId,
    MeterNotFound,
    ResourceNotFound,
    IdentityMismatch,
    AssignedElsewhere,
    Database,
}

/// Associates a resource only when its utility and commodity match.
/// Locking the resource row serializes competing assignments, including
/// assignments to different physical meters.
pub async fn associate_meter_resource(
    pool: &MySqlPool,
    meter_id: u64,
    resource_id: u64,
) -> Result<AssociationOutcome, AssociationError> {
    use AssociationError as E;
    if meter_id == 0 || resource_id == 0 {
        return Err(E::InvalidId);
    }
    let mut tx = pool.begin().await.map_err(|_| E::Database)?;
    // Lock the resource itself, not just an optional association row.
    let resource: Option<(u64, Option<u32>)> = sqlx::query_as(
        "SELECT src.data_custodian_id, rt.commodity \
         FROM meter_reading mr \
         JOIN usage_point up ON up.id = mr.usage_point_id \
         JOIN acquisition_source src \
           ON src.id = up.acquisition_source_id \
         JOIN reading_type rt ON rt.id = mr.reading_type_id \
         WHERE mr.id = ? FOR UPDATE",
    )
    .bind(resource_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| E::Database)?;
    let (custodian, commodity) = resource.ok_or(E::ResourceNotFound)?;
    let meter: Option<(u64, u32)> = sqlx::query_as(
        "SELECT data_custodian_id, commodity FROM physical_meter \
         WHERE id = ?",
    )
    .bind(meter_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| E::Database)?;
    let (meter_custodian, meter_commodity) = meter.ok_or(E::MeterNotFound)?;
    if custodian != meter_custodian || commodity != Some(meter_commodity) {
        return Err(E::IdentityMismatch);
    }
    let current: Option<(u64,)> = sqlx::query_as(
        "SELECT physical_meter_id FROM physical_meter_resource \
         WHERE meter_reading_id = ?",
    )
    .bind(resource_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| E::Database)?;
    let outcome = match current {
        Some((id,)) if id == meter_id => AssociationOutcome::AlreadyAssociated,
        Some(_) => return Err(E::AssignedElsewhere),
        None => {
            sqlx::query(
                "INSERT INTO physical_meter_resource \
                 (physical_meter_id, meter_reading_id) VALUES (?, ?)",
            )
            .bind(meter_id)
            .bind(resource_id)
            .execute(&mut *tx)
            .await
            .map_err(|_| E::Database)?;
            AssociationOutcome::Associated
        }
    };
    tx.commit().await.map_err(|_| E::Database)?;
    Ok(outcome)
}
