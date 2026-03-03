use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CalendarEventSeen::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CalendarEventSeen::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(CalendarEventSeen::Provider)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CalendarEventSeen::CalendarId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CalendarEventSeen::EventId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CalendarEventSeen::EventStart)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CalendarEventSeen::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("uk-calendar-event-seen")
                    .table(CalendarEventSeen::Table)
                    .col(CalendarEventSeen::Provider)
                    .col(CalendarEventSeen::CalendarId)
                    .col(CalendarEventSeen::EventId)
                    .col(CalendarEventSeen::EventStart)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CalendarEventSeen::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum CalendarEventSeen {
    Table,
    Id,
    Provider,
    CalendarId,
    EventId,
    EventStart,
    CreatedAt,
}
