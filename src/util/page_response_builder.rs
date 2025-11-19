use serde::Serialize;
use serde_json::json;

pub fn page_response_builder<T: Serialize>(
    page: i32,
    page_size: i32,
    data: &[T],
) -> serde_json::Value {
    let start = ((page - 1) * page_size) as usize;
    let end = (start + page_size as usize).min(data.len());
    let paginated_list = data.get(start..end).unwrap();

    let total_items = data.len() as i32;
    let total_pages = (total_items + page_size - 1) / page_size;

    json!({
        "data": paginated_list,
        "page": page,
        "page_size": page_size,
        "total_items": total_items,
        "total_pages": total_pages,
    })
}

pub fn page_response_extra_builder<T: Serialize>(
    page: i32,
    page_size: i32,
    data: &[T],
    extra_data: serde_json::Value,
) -> serde_json::Value {
    let start = ((page - 1) * page_size) as usize;
    let end = (start + page_size as usize).min(data.len());
    let paginated_list = data.get(start..end).unwrap();

    let total_items = data.len() as i32;
    let total_pages = (total_items + page_size - 1) / page_size;

    json!({
        "data": paginated_list,
        "page": page,
        "page_size": page_size,
        "total_items": total_items,
        "total_pages": total_pages,
        "extra_data": extra_data
    })
}