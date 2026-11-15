# launcher/usage-history Specification

## Purpose
Registra y consulta cuántas veces y cuándo se ha lanzado cada aplicación, conservando el dato entre sesiones.

## Requirements

### Requirement: Registro de lanzamientos
The system SHALL record every successful application launch, incrementing a per-item launch counter and updating its last-used timestamp.

#### Scenario: Registrar un lanzamiento
- **WHEN** el usuario lanza correctamente una aplicación
- **THEN** el contador de lanzamientos de esa aplicación aumenta en uno y su marca de último uso se actualiza

### Requirement: Persistencia entre sesiones
The system SHALL persist history in a SQLite database located in the application's config directory, using WAL mode, and SHALL load it on startup.

#### Scenario: Historial conservado
- **WHEN** el usuario reinicia LiLa tras haber lanzado aplicaciones
- **THEN** el historial de uso previo sigue disponible

### Requirement: Esquema versionado y migraciones idempotentes
The system SHALL store a schema version and apply idempotent migrations when opening the database.

#### Scenario: Base de datos nueva
- **WHEN** no existe la base de datos de historial
- **THEN** el sistema crea el esquema en la versión actual sin error

#### Scenario: Base de datos en una versión anterior
- **WHEN** la base de datos está en una versión de esquema anterior
- **THEN** el sistema aplica las migraciones necesarias y conserva los datos existentes

### Requirement: Consulta de la puntuación de uso
The system SHALL expose, for a given item identifier, a usage score combining frequency and recency.

#### Scenario: Aplicación nunca usada
- **WHEN** se consulta la puntuación de una aplicación sin lanzamientos registrados
- **THEN** la puntuación devuelta es la mínima
