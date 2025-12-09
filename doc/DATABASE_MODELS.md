# Modelos y Estructura de la Base de Datos

## Índice
1. [Visión General](#visión-general)
2. [Diagrama Entidad-Relación](#diagrama-entidad-relación)
3. [Modelos de Datos](#modelos-de-datos)
4. [Relaciones Principales](#relaciones-principales)
5. [Índices y Restricciones](#índices-y-restricciones)
6. [Migración y Evolución](#migración-y-evolución)

---

## Visión General

ReqMan utiliza PostgreSQL como base de datos relacional con Diesel como ORM. El esquema está diseñado para gestionar proyectos de gestión de requisitos con trazabilidad completa entre requisitos y pruebas.

### Tecnologías
- **Base de datos**: PostgreSQL
- **ORM**: Diesel 2.x
- **Migraciones**: Diesel CLI
- **Lenguaje**: Rust

---

## Diagramas Entidad-Relación

### 1. Diagrama Principal: Core Entities

```mermaid
erDiagram
    PROJECTS ||--o{ REQUIREMENTS : contains
    PROJECTS ||--o{ TESTS : contains
    PROJECTS ||--o{ PROJECT_MEMBERS : has
    PROJECTS }o--|| PROJECT_STATUS : "has status"
    PROJECTS }o--|| USERS : "owned by"
    
    USERS ||--o{ REQUIREMENTS : authors
    USERS ||--o{ REQUIREMENTS : reviews
    USERS ||--o{ PROJECT_MEMBERS : "member of"
    USERS ||--o{ LOGS : performs
    
    REQUIREMENTS }o--|| REQUIREMENT_STATUS : "has status"
    REQUIREMENTS }o--|| CATEGORIES : "belongs to"
    REQUIREMENTS }o--|| APPLICABILITY : "applies to"
    REQUIREMENTS }o--|| VERIFICATION : "verified by"
    REQUIREMENTS }o--o| REQUIREMENTS : "parent of"
    REQUIREMENTS ||--o{ MATRIX : "traced by"
    
    TESTS }o--|| TEST_STATUS : "has status"
    TESTS }o--o| TESTS : "parent of"
    TESTS ||--o{ MATRIX : "traced by"
    
    MATRIX }o--|| REQUIREMENTS : traces
    MATRIX }o--|| TESTS : traces
    
    PROJECTS {
        int id PK
        varchar name
        text description
        timestamp creation_date
        timestamp update_date
        int status_id FK
        int owner_id FK
    }
    
    USERS {
        int id PK
        varchar username
        varchar name
        varchar email
        varchar password_hash
        bool is_admin
        timestamp creation_date
        timestamp last_login
    }
    
    REQUIREMENTS {
        int id PK
        varchar title
        text description
        varchar reference_code UK
        int status_id FK
        int author_id FK
        int reviewer_id FK
        int category_id FK
        int applicability_id FK
        int verification_method_id FK
        int parent_id FK
        int project_id FK
        timestamp creation_date
        timestamp update_date
        timestamp deadline_date
    }
    
    TESTS {
        int id PK
        varchar name
        varchar reference_code UK
        text description
        varchar source
        int status_id FK
        int parent_id FK
        int project_id FK
    }
    
    MATRIX {
        int req_id PK,FK
        int test_id PK,FK
        int project_id FK
        timestamp creation_date
    }
    
    PROJECT_MEMBERS {
        int project_id PK,FK
        int user_id PK,FK
        int role
        timestamp created_at
        timestamp updated_at
    }
```

### 2. Entidades de Configuración (Tagged Entities)

Todas estas entidades comparten la misma estructura y son personalizables por proyecto:

```mermaid
erDiagram
    PROJECTS ||--o{ CATEGORIES : has
    PROJECTS ||--o{ APPLICABILITY : has
    PROJECTS ||--o{ REQUIREMENT_STATUS : has
    PROJECTS ||--o{ TEST_STATUS : has
    PROJECTS ||--o{ VERIFICATION : has
    
    PROJECTS {
        int id PK
        varchar name
    }
    
    CATEGORIES {
        int id PK
        varchar title
        text description
        varchar tag
        int project_id FK
    }
    
    APPLICABILITY {
        int id PK
        varchar title
        text description
        varchar tag
        int project_id FK
    }
    
    REQUIREMENT_STATUS {
        int id PK
        varchar title
        text description
        varchar tag
        int project_id FK
    }
    
    TEST_STATUS {
        int id PK
        varchar title
        text description
        varchar tag
        int project_id FK
    }
    
    VERIFICATION {
        int id PK
        varchar title
        text description
        varchar tag
        int project_id FK
    }
    
    PROJECT_STATUS {
        int id PK
        varchar name
        text description
        timestamp created_at
    }
```

### 3. Sistema de Auditoría

```mermaid
erDiagram
    USERS ||--o{ LOGS : performs
    PROJECTS ||--o{ LOGS : tracks
    
    LOGS {
        int log_id PK
        int user_id FK
        varchar action_type
        varchar entity_type
        int entity_id
        int project_id FK
        text old_values
        text new_values
        text description
        varchar ip_address
        text user_agent
        timestamp created_at
    }
    
    USERS {
        int id PK
        varchar username
        varchar name
    }
    
    PROJECTS {
        int id PK
        varchar name
    }
```

---

## Modelos de Datos

Para el detalle completo de cada tabla consulta el esquema en [`src/schema.rs`](../src/schema.rs).

### Entidades Principales

#### **Projects**
Agrupa requisitos, tests y configuraciones. Tiene relación con `ProjectStatus` para gestión del ciclo de vida.

**Campos clave**: `id`, `name`, `description`, `status_id`, `owner_id`

#### **Requirements**
Requisitos del sistema con trazabilidad completa, jerarquías (campo `parent_id`), y metadatos como estado, categoría, aplicabilidad y método de verificación.

**Campos clave**: `id`, `title`, `reference_code` (UNIQUE), `status_id`, `author_id`, `reviewer_id`, `project_id`, `parent_id`

#### **Tests**
Casos de prueba vinculados a requisitos mediante la tabla `Matrix`. Soporta jerarquías.

**Campos clave**: `id`, `name`, `reference_code` (UNIQUE), `status_id`, `project_id`, `parent_id`

#### **Users**
Usuarios del sistema con autenticación. Campo `is_admin` para permisos globales.

**Campos clave**: `id`, `username`, `email`, `password_hash`, `is_admin`

⚠️ **Seguridad**: `password_hash` nunca debe exponerse en APIs (protegido con `#[serde(skip_serializing)]`)

#### **Matrix (Trazabilidad)**
Tabla de enlace N:M entre requisitos y tests. Clave primaria compuesta: (`req_id`, `test_id`)

#### **ProjectMembers**
Gestión de acceso por proyecto con roles (0=viewer, 1=editor, 2=admin). Clave primaria: (`project_id`, `user_id`)

### Entidades de Configuración (Tagged Entities)

Las siguientes entidades comparten estructura y son personalizables por proyecto:

- **Categories**: Clasificación de requisitos
- **Applicability**: Contextos de aplicación
- **RequirementStatus**: Estados de requisitos
- **TestStatus**: Estados de tests
- **Verification**: Métodos de verificación

**Estructura común**: `id`, `title`, `description`, `tag`, `project_id`

### Auditoría

#### **Logs**
Registro completo de todas las acciones del sistema (CREATE, UPDATE, DELETE, LOGIN, etc.) con valores antiguos/nuevos en JSON.

**Campos clave**: `log_id`, `user_id`, `action_type`, `entity_type`, `entity_id`, `created_at`

---

## Relaciones Principales

### Jerarquías de Proyectos
```
Projects
  ├── Requirements (con jerarquías internas vía parent_id)
  ├── Tests (con jerarquías internas vía parent_id)
  ├── Categories
  ├── Applicability
  ├── RequirementStatus
  ├── TestStatus
  ├── Verification
  └── Matrix (trazabilidad req-test)
```

### Trazabilidad
- **Requirements ↔ Tests**: Relación N:M mediante tabla `Matrix`
- **Projects ↔ Users**: Relación N:M mediante `ProjectMembers` con roles

### Restricciones de Integridad
- Claves foráneas con `ON DELETE CASCADE` (excepto `status_id` y `parent_id` que usan `SET NULL`)
- `reference_code` único en `requirements` y `tests`
- Restricciones CHECK para prevenir auto-referencias y validar roles

---

## Índices y Restricciones

### Índices de Rendimiento
- **Por proyecto**: `requirements`, `tests`, `matrix`, `logs` todos indexados por `project_id`
- **Por estado**: `requirements.status_id`, `tests.status_id`
- **Jerarquías**: `requirements.parent_id`, `tests.parent_id`
- **Auditoría**: `logs.user_id`, `logs.created_at`, `logs(entity_type, entity_id)`
- **Trazabilidad**: `matrix.req_id`, `matrix.test_id`

Ver migración `2025-11-23-000006_add_performance_indexes` para detalles.

### Restricciones Principales
- **UNIQUE**: `requirements.reference_code`, `tests.reference_code`
- **CHECK**: Validación de roles (0-2), prevención de auto-referencias en jerarquías
- **FOREIGN KEYS**: Mayoritariamente con `ON DELETE CASCADE`, excepto `status_id` y `parent_id` que usan `SET NULL`

Ver migraciones `2025-11-23-000004` (FKs) y `2025-11-23-000005` (CHECKs) para detalles.

---

## Migración y Evolución

El proyecto utiliza Diesel CLI para gestionar migraciones.

### Historial de Cambios Clave

| Fecha | Cambio |
|-------|--------|
| 2022-11-07 | Creación inicial (`requirements`, `users`, `tests`, `matrix`) |
| 2025-08-03 | Añadida `applicability` y `justification` |
| 2025-08-06 | Sistema multi-proyecto con `projects` y `project_members` |
| 2025-09-06 | División de tablas de estado por proyecto |
| 2025-11-23 | Suite de mejoras: `project_status`, FKs completas, restricciones CHECK, índices de rendimiento |

### Comandos Diesel

```bash
diesel migration generate nombre_migracion  # Crear
diesel migration run                         # Aplicar
diesel migration revert                      # Revertir
diesel print-schema > src/schema.rs         # Regenerar schema
```

Ver carpeta [`migrations/`](../migrations/) para todas las migraciones.

---

## Recursos Adicionales

- **Schema Diesel**: [`src/schema.rs`](../src/schema.rs)
- **Modelos Rust**: [`src/models/entities.rs`](../src/models/entities.rs)
- **Migraciones**: [`migrations/`](../migrations/)
- **Setup de BD**: [`DATABASE_SETUP_README.md`](../DATABASE_SETUP_README.md)

---

**Última actualización**: 9 de diciembre de 2025
