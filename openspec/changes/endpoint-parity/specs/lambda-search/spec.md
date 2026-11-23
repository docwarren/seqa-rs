# Spec Delta

## REMOVED Requirements

### Requirement: Search takes a list of coordinates
**Reason**: Superseded by `http-api`, which defines search for both `seqa_axum` and `seqa_lambda`.
**Migration**: See `http-api` "Search request" (the "Coordinates must be a list" scenario) and "Search response" (the "Two coordinates" scenario).

### Requirement: Empty coordinates rejected
**Reason**: Superseded by `http-api`.
**Migration**: See `http-api` "Search request validation" (the "Empty coordinates" scenario).

### Requirement: Results paired with coordinates
**Reason**: Superseded by `http-api`.
**Migration**: See `http-api` "Search response".

### Requirement: Header only when asked
**Reason**: Superseded by `http-api`.
**Migration**: See `http-api` "Search response" (the "Header requested" and "Header not requested" scenarios).
