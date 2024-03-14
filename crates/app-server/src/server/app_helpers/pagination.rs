use sea_orm::{ConnectionTrait, DbErr, Paginator, SelectorTrait};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginationQuery {
    page: Option<u64>,
    page_size: Option<u64>,
}
impl PaginationQuery {
    pub fn page(&self) -> u64 {
        self.page.unwrap_or(1).max(1) - 1
    }

    pub fn page_size(&self) -> u64 {
        self.page_size.unwrap_or(10).clamp(1, 100)
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Paginated<T> {
    pub items: Vec<T>,
    pub pagination: PaginationMeta,
}
impl<T> Paginated<T> {
    pub const fn new(items: Vec<T>, pagination: PaginationMeta) -> Self {
        Self { items, pagination }
    }

    pub fn items_into<I>(self) -> Paginated<I>
    where
        I: From<T>,
    {
        Paginated {
            items: self.items.into_iter().map(Into::into).collect(),
            pagination: self.pagination,
        }
    }
}
impl<'db, S: SelectorTrait + 'db + Send + Sync> Paginated<S> {
    pub async fn from_paginator<C>(
        paginator: Paginator<'db, C, S>,
        page: u64,
        page_size: u64,
    ) -> Result<Paginated<S::Item>, DbErr>
    where
        C: ConnectionTrait,
        S::Item: Send,
    {
        let (items, pagination) =
            futures::try_join!(paginator.fetch_page(page), paginator.num_items_and_pages())?;

        let pagination = PaginationMeta {
            page: page + 1,
            page_size,
            total: pagination.number_of_items,
        };

        Ok(Paginated::new(items, pagination))
    }

    pub async fn from_paginator_query<C>(
        paginator: Paginator<'db, C, S>,
        query: PaginationQuery,
    ) -> Result<Paginated<S::Item>, DbErr>
    where
        C: ConnectionTrait,
        S::Item: Send,
    {
        let page = query.page();
        let page_size = query.page_size();

        Self::from_paginator(paginator, page, page_size).await
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginationMeta {
    pub page: u64,
    pub page_size: u64,
    pub total: u64,
}
