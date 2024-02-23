use sea_orm_migration::sea_query::{Iden, IdenList, Index, IndexCreateStatement, IntoIndexColumn};
use sha2::{Digest, Sha256};

macro_rules! debug_print {
    ($($arg:tt)*) => {
        if crate::DEBUG {
            println!(
                "\n==========================\n{}\n==========================\n",
                $($arg)*,
            );
        }
    }
}

pub enum GenKeyType {
    ForeignKey,
    Index,
    Trigger,
}
impl GenKeyType {
    pub fn gen_name<TTable, TColumns>(&self, table: &TTable, cols: TColumns) -> String
    where
        TTable: ToString,
        TColumns: IdenList,
    {
        generate_name(
            self,
            table,
            cols.into_iter().map(|x| x.to_string()).collect(),
        )
    }
}
impl ToString for GenKeyType {
    fn to_string(&self) -> String {
        match self {
            Self::ForeignKey => "fk",
            Self::Index => "idx",
            Self::Trigger => "trigger",
        }
        .to_string()
    }
}

pub fn generate_name<TTable, TColumn>(
    for_type: &GenKeyType,
    table: &TTable,
    cols: Vec<TColumn>,
) -> String
where
    TTable: ToString,
    TColumn: ToString,
{
    let hash = hash_list(&{
        let mut parts = vec![for_type.to_string(), table.to_string()];
        parts.append(&mut cols.into_iter().map(|x| x.to_string()).collect::<Vec<_>>());
        parts
    });

    format!("{}-{}", for_type.to_string(), hash)
}

pub fn generate_index<TTable, TColumn>(table: TTable, cols: Vec<TColumn>) -> IndexCreateStatement
where
    TTable: Iden + 'static,
    TColumn: IntoIndexColumn,
{
    let mut column_names = vec![];
    let table_str = table.to_string();
    let mut stmt = Index::create();
    stmt.table(table);
    for col in cols.into_iter() {
        let col = col.into_index_column();
        column_names.push(format!("{:?}", &col));
        stmt.col(col);
    }
    let name = generate_name(&GenKeyType::Index, &table_str, column_names);

    stmt.name(name);
    stmt
}

pub fn hash_list<T>(list: &[T]) -> String
where
    T: ToString,
{
    let mut hasher = Sha256::new();
    for item in list {
        hasher.update(item.to_string().as_bytes());
    }
    let hash = hasher.finalize();
    hex::encode(hash)
}
