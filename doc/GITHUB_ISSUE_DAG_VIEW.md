# Nova vista DAG de la jerarquia de requirements

## Resum

Implementar una nova vista a la pàgina de requirements que mostri la jerarquia DAG (Directed Acyclic Graph) dels requirements, amb dues variants d’implementació en branques diferents: una amb **D3.js** i una amb **Cytoscape.js**.

## Objectius

- Mostrar la jerarquia de requirements com a DAG (arrels sense parents, fills carregats a demanda).
- A cada node: una **petita carta** amb el **títol** i la **key** (reference_code) del requirement.
- La carta ha de tenir el **color segons l’estatus** del requirement (tag_color del projecte).
- **Càrrega progressiva**: en obrir la vista només es mostren les entitats de **primer nivell** (arrels); en **clic a un node** es carreguen i es mostren les entitats filles.
- **Clic al nom** de l’entitat ha d’obrir la informació detallada de l’entitat (`/p/<project_id>/requirements/show/<requirement_id>`).

## Branques proposades

| Branca | Biblioteca | Descripció |
|--------|------------|------------|
| **Branca 1** | D3.js | Vista DAG amb D3 (force layout o d3-dag), nodes com a cartes (foreignObject o g+rect), expandir en clic, link al detall en clic al nom. |
| **Branca 2** | Cytoscape.js | Vista DAG amb Cytoscape (breadthfirst/dagre), nodes amb label i color d’estat, mateix comportament d’expandir i obrir detall. |

## Backend (comú a les dues branques)

- **`GET /api/projects/<project_id>/requirements/roots`**  
  Retorna els requirements del projecte sense cap enllaç parent (arrels del DAG).
- **`GET /api/projects/<project_id>/requirements/<parent_id>/children`**  
  Retorna els fills directes d’un requirement (per càrrega en expandir).
- Permisos: mateix que la llista de requirements del projecte (ViewRequirements).

## Frontend (comú)

- Nou botó **DAG** al selector de vista (Table / Card / Tree / **DAG**) a la pàgina de requirements.
- Nova secció `#dagView` amb contenidor per al gràfic, missatges buit/càrrega i (opcional) selector d’engine D3 vs Cytoscape.
- Suport a `?view=dag` a la URL per obrir directament la vista DAG.
- La vista ha d’ocupar **més espai** a la pàgina (amplada gran, alçada mínima tipus 60vh per al gràfic).

## UX

- **Expandir vs obrir detall**:  
  - Clic a la carta / botó d’expandir → carrega i mostra fills.  
  - Clic al nom/reference (link) → obre la pàgina de detall de l’entitat.
- **Estats buits**: missatge clar quan no hi ha arrels o quan un node no té fills.
- **Layout**: D3 (force/dagre) i Cytoscape (breadthfirst/dagre) per a un DAG llegible.

## Referència

- Pla d’implementació: `.cursor/plans/` (DAG view D3 vs Cytoscape).
- Backend existent: `TraceSummary`, `get_children_by_parent_and_project`, `requirement_version_links`.
- Colors d’estat: `inline_edit_config_json` (statuses amb `tag_color`) a la pàgina de requirements.

## Criteris d’acceptació

- [ ] Es pot seleccionar la vista DAG des del selector de vista.
- [ ] Es mostren només les arrels en carregar; en clicar un node es carreguen els fills.
- [ ] Cada node es veu com a carta amb títol i key i color segons l’estatus.
- [ ] Clic al nom/key obre la pàgina de detall del requirement.
- [ ] La vista ocupa un espai ampli a la pàgina (gràfic gran i llegible).
- [ ] (Opcional) Es pot triar entre D3 i Cytoscape des de la mateixa vista DAG.
