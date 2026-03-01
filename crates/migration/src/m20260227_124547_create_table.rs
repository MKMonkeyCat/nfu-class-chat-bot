use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(GuildMember::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(GuildMember::UserId)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(GuildMember::Name).string().not_null())
                    .col(ColumnDef::new(GuildMember::EmployeeId).string().not_null())
                    .col(ColumnDef::new(GuildMember::Identity).string().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Announcement::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Announcement::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Announcement::Category).string().not_null())
                    .col(ColumnDef::new(Announcement::SourceName).string().not_null())
                    .col(ColumnDef::new(Announcement::Title).string().not_null())
                    .col(ColumnDef::new(Announcement::Url).string().not_null())
                    .col(ColumnDef::new(Announcement::Content).text().not_null())
                    .col(ColumnDef::new(Announcement::Time).string().not_null())
                    .col(ColumnDef::new(Announcement::Tags).json().not_null())
                    .col(
                        ColumnDef::new(Announcement::ImplementationAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Announcement::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(Announcement::Simhash).string().not_null())
                    .col(ColumnDef::new(Announcement::Chunk0).unsigned().not_null())
                    .col(ColumnDef::new(Announcement::Chunk1).unsigned().not_null())
                    .col(ColumnDef::new(Announcement::Chunk2).unsigned().not_null())
                    .col(ColumnDef::new(Announcement::Chunk3).unsigned().not_null())
                    .to_owned(),
            )
            .await?;

        let indexes = [
            (
                Announcement::ImplementationAt,
                "idx-announcement-implementation_at",
            ),
            (Announcement::Chunk0, "idx-announcement-chunk0"),
            (Announcement::Chunk1, "idx-announcement-chunk1"),
            (Announcement::Chunk2, "idx-announcement-chunk2"),
            (Announcement::Chunk3, "idx-announcement-chunk3"),
        ];

        for (col, name) in indexes {
            manager
                .create_index(
                    Index::create()
                        .name(name)
                        .table(Announcement::Table)
                        .col(col)
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Announcement::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(GuildMember::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum GuildMember {
    Table,
    UserId,
    Name,
    EmployeeId,
    Identity,
}

#[derive(DeriveIden)]
enum Announcement {
    Table,
    Id,
    Category,
    SourceName,
    Title,
    Url,
    Content,
    Time,
    Tags,
    ImplementationAt,
    CreatedAt,
    Simhash,
    Chunk0,
    Chunk1,
    Chunk2,
    Chunk3,
}
