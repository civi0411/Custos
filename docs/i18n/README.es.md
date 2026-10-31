<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../assets/banner-dark.png">
  <source media="(prefers-color-scheme: light)" srcset="../assets/banner.png">
  <img alt="Custos" src="../assets/banner.png" width="100%">
</picture>

[ EN ](../../README.md) · [ VI ](README.vi.md) · [ DE ](README.de.md) · [ ZH ](README.zh.md) · [ JA ](README.ja.md) · [ KO ](README.ko.md) · [ ES ](README.es.md)

</div>

> **Entorno de Desarrollo de Agentes Supervisados (SADE) — Local-First, Basado en Evidencia, Gobernado por Humanos.**

Custos es un entorno de desarrollo y ejecución de prioridad local (local-first) para agentes autónomos en ingeniería de software, investigación científica y asistencia en flujos de trabajo personales. A diferencia de simples envoltorios de LLM o sistemas multiagente opacos, Custos impone persistencia de estado determinista, puertas de capacidad auditadas criptográficamente, compilación de contexto vinculada al origen y la exigencia de pruebas empíricas antes de reconocer cualquier tarea como completada.

---

## Motivación Fundamental y Desafíos

Los sistemas de agentes contemporáneos ofrecen notables capacidades de generación, pero adolecen de fallos arquitectónicos fundamentales:

- **Pérdida de Contexto Efímero:** El contexto queda atrapado en silos propietarios de proveedores y se descarta al cambiar de herramientas o reiniciar sesiones.
- **Afirmaciones No Verificadas:** Las declaraciones de finalización emitidas por los modelos se aceptan ciegamente sin verificación objetiva ni pruebas reproducibles.
- **Efectos Secundarios Incontrolados:** Las mutaciones externas (edición de archivos, peticiones de red, comandos del sistema) se ejecutan sin puertas de capacidad estrictas ni registros de auditoría.
- **Estado Frágil:** Las caídas del sistema, límites de tasa y desbordamientos de contexto destruyen el trabajo en curso sin mecanismos deterministas de recuperación.
- **Erosión de la Soberanía Humana:** Los bucles multiagente opacos socavan la supervisión humana y suprimen puntos de decisión críticos.

Custos resuelve estos problemas introduciendo un modelo operativo basado en evidencia, donde la soberanía humana es absoluta y cada efecto externo está estrictamente gobernado.

---

## Tres Pilares No Negociables

| Pilar | Principio Fundamental | Garantía Arquitectónica |
|---|---|---|
| **P1 — Tarea como Entidad de Primer Nivel** | Las tareas existen independientemente de sesiones y conexiones | Persistencia SQLite WAL duradera, recuperación determinista tras caídas y libro de órdenes idempotente. |
| **P2 — Propuesta ≠ Decisión ≠ Efecto** | El modelo propone; el núcleo autoriza; la pasarela ejecuta | El modelo IA no confiable solo emite un `ActionIntent`; el núcleo firma un `ExecutionPermit` de un solo uso; el sandbox aplica las puertas de capacidad vía Outbox. |
| **P3 — Finalización Basada en Evidencia** | Ninguna tarea se completa sin prueba empírica | Las condiciones de aceptación solo se aprueban con artefactos objetivos verificables (diffs, trazas de pruebas, auditorías de citas); cero finalizaciones ficticias. |

---

## Mecánica Operativa

```mermaid
flowchart LR
    Human[Operador Humano] --> Session[Sesión de Workbench]
    Session --> Task[Tarea Persistente]
    Task --> Context[Compilación de Contexto]
    Context --> Worker[Modelo / Harness Worker]
    Worker --> Intent[Intención de Acción ActionIntent]
    Intent --> Gate[Puerta de Autoridad y Permiso]
    Gate --> Effect[Efecto Aislado en Sandbox]
    Effect --> Evidence[Evidencia Objetiva Evidence]
    Evidence --> Outcome[Finalización Verificada]
```

---

## Tres Lentes de Dominio Especializadas

Custos proporciona modelos de interacción y motores de verificación adaptados a distintos dominios cognitivos:

### 1. Lente de Ingeniería (Engineering Lens - Banco de Trabajo de Programación)
- **Espacios de Trabajo de Ejecución de 3 Vías (3-Path Workspaces):** Aísla de forma segura el repositorio canónico, el fork de preparación (staging) y el sandbox de verificación.
- **Paquetes de Parches Atómicos (Atomic Patch Bundles):** Exige diffs estructurados con validación sintáctica y de lint automatizada antes de la preparación.
- **Aislamiento de Arrendamiento de Árbol de Trabajo (Worktree Lease Isolation):** Protege mediante barreras (fencing) las escrituras concurrentes en el mismo árbol de trabajo para evitar condiciones de carrera y pérdida de datos.

### 2. Lente de Investigación (Research Lens - Banco de Trabajo de Investigación)
- **Jerarquía de Fundamentación de Afirmaciones (Claim Grounding Hierarchy):** Establece cuatro niveles de rigor epistémico: `L0 Sin Fundamento`, `L1 Con Respaldo de Citas`, `L2 Revisado por Pares`, `L3 Reproducido Empíricamente`.
- **Oráculo de Cobertura de Citas (Citation Coverage Oracle):** Contrasta rigurosamente con literatura y DOI publicados, rechazando declaraciones de finalización infundadas.
- **Estudio de Síntesis (Synthesis Studio):** Transforma revisiones bibliográficas multifuente en hipótesis contrastables y lotes de experimentos reproducibles.

### 3. Lente de Asistente (Assistant Lens - Asistente y Banco de Trabajo Compartido)
- **Cola de Aprobación de Bandeja de Salida (Outbox Approval Queue):** Aísla comunicaciones externas y mutaciones críticas tras una cola de aprobación pendiente duradera.
- **Autorizaciones Permanentes (Standing Grants):** Permite la delegación basada en políticas para tareas rutinarias, preservando la intervención humana en acciones de alto riesgo.
- **Entorno de Ejecución Integrado:** Acceso unificado y seguro a navegadores aislados, terminales PTY, flotas de servidores remotos y notas persistentes.

---

## 10 Invariantes Inviolables del Sistema

1. **INV-01 (Soberanía Local-First):** Todo el estado, contratos, tareas y telemetría residen exclusivamente en el disco local.
2. **INV-02 (Cero Atomicidad Ilusoria):** Todo cambio de estado se confirma en SQLite antes de despachar acciones externas.
3. **INV-03 (Permiso Previo al Efecto):** Ninguna mutación externa se ejecuta sin un `ExecutionPermit` criptográficamente vinculado.
4. **INV-04 (Vinculación de Identidad del Actor):** La identidad del actor se vincula desde capas de transporte autenticadas, nunca desde cargas autodeclaradas.
5. **INV-05 (Evidencia por Encima de Afirmaciones):** La aceptación de tareas exige artefactos empíricos verificados, no meras confirmaciones del modelo.
6. **INV-06 (Veracidad Explícita en la Interfaz):** La interfaz entra de inmediato en modo de fallo seguro (`unavailable`/`degraded`) si los servicios de backend fallan.
7. **INV-07 (Aislamiento de Arrendamiento de Árbol de Trabajo):** Las escrituras concurrentes en el mismo árbol de trabajo se aíslan mediante barreras para evitar pérdidas de datos.
8. **INV-08 (Límites Estrictos de Recursos):** La ejecución de procesos, el búfer de líneas y la poda de tokens respetan límites estrictos en bytes.
9. **INV-09 (Transportes Auditados Integralmente):** Toda mutación pasa por un Command Ledger idempotente con verificación de resumen criptográfico contra repetición.
10. **INV-10 (Supremacía Humana):** El operador humano retiene el derecho irrestricto de pausar, redirigir o abortar tareas en cualquier fase de su ciclo de vida.

---

## Crates del Espacio de Trabajo y Topología Arquitectónica

Custos aísla estrictamente la lógica de dominio pura de los efectos secundarios a través de paquetes de Cargo por capas:

| Capa | Ruta del Crate | Nombre del Paquete | Responsabilidad Principal |
|---|---|---|---|
| **Layer 0 (Domain)** | `crates/custos-domain` | `custos-domain` | Entidades puras de dominio, modelos de datos, máquinas de estado e invariantes Zero-I/O. |
| **Layer 1 (Core)** | `crates/custos-core` | `custos-core` | Servicio de orquestación de tareas, motor de autoridad y verificadores de finalización. |
| | `crates/custos-persistence` | `custos-persistence` | Persistencia SQLite WAL, patrón Outbox, transiciones de estado CAS `state_version` y recuperación de caídas. |
| | `crates/custos-provider` | `custos-provider` | Contratos neutrales `ModelPort`, analizadores SSE en flujo y medidores de tokens. |
| **Layer 2 (Runtime & Adapters)** | `crates/custos-bridge` | `custos-bridge` | Promoción de sesión a tarea (Session-to-Task), redirección en vuelo (Steering) y enrutamiento IPC. |
| | `crates/custos-runtime` | `custos-runtime` | Bucle cognitivo S1/S2, compresión de contexto (RTK), capas de memoria y planificadores de ejecución. |
| | `crates/custos-adapters` | `custos-adapters` | Sandboxes del SO, adaptador de clientes MCP, sidecars de inferencia local, árboles Git y traductores de protocolos. |
| **Layer 3 (Domain Packs)** | `crates/custos-packs` | `custos-packs` | Flujos de trabajo especializados: paquetes de Ingeniería (Engineering), Investigación (Research) y Asistente (Assistant). |
| **Layer 4 (Composition & Hosts)** | `crates/custos-daemon` | `custos-daemon` | Raíz de composición única (Composition Root), servicio daemon en segundo plano y HTTP/JSON-RPC local. |
| | `crates/custos-sdk` | `custos-sdk` | Objetos de transferencia de datos de cliente (DTO), bindings UniFFI y tipos de transporte ACP. |
| | `crates/custos-app/cli` | `custos-app-cli` | Interfaz de terminal para el operador basada en `ratatui`. |
| | `crates/custos-app/desktop` | `custos-app-desktop` | Shell anfitrión nativo Tauri 2 con integración del frontend `ui/desktop`. |

---

## Interfaz de Usuario de Escritorio

Custos Desktop está construido con **Vite, React 19 y Tauri 2**:

- **Jerarquía de 3 Niveles:** Navegación fluida a través de `Proyecto -> Espacio de Trabajo -> Árbol de Trabajo (Project -> Workspace -> Worktree)`.
- **Sistema de Tokens Graphite JetBlack:** Tema oscuro moderno de alto contraste optimizado para sesiones de trabajo prolongadas.
- **Bancos de Trabajo Dedicados (Workbenches):**
  - **Ingeniería (Engineering):** Archivos y Cambios, Evidencia Objetiva y Gestor de Worktrees.
  - **Investigación (Research):** Estudio de Síntesis, Hallazgos y Libro de Ejecuciones.
  - **Asistente y Compartido (Assistant & Shared):** Navegador Aislado, Terminal, Flota Remota y Notas.
- **Política Zero-Mock:** Todas las métricas de la interfaz reflejan telemetría real del daemon y conmutan a fallo cerrado en caso de degradación.

---

## Concentradores de Protocolo (Hubs) y Supervisión de Arneses Nativos

El daemon de Custos actúa como una pasarela central de interoperabilidad a través de 6 concentradores especializados:

- **Model Hub:** Enruta peticiones dinámicamente entre motores locales (GGUF, llama.cpp) y proveedores en la nube (Anthropic, OpenAI, Gemini, Bedrock) con conmutación por error automática.
- **Tool Hub:** Gobierna clientes de Model Context Protocol (MCP) y herramientas del sistema aisladas en sandbox.
- **Agent Hub:** Coordina protocolos Agente a Agente (A2A) y flujos de trabajo de Agent Communication Protocol (ACP).
- **Storage Hub:** Gestiona la persistencia SQLite WAL, almacenamiento de blobs e índices estructurados.
- **Event Hub:** Distribuye telemetría de auditoría de alto rendimiento y eventos del ciclo de vida.
- **Operator Hub:** Proporciona puntos de enlace locales autenticados para CLI y escritorio.

### Supervisión de Arneses Nativos
Custos envuelve y supervisa arneses de programación externos sin alterar sus mecanismos internos:
- **Claude Code:** Aislamiento de subprocesos Stdio y envoltura segura de comandos.
- **Codex:** Autocompletado de código en sandbox y ejecución automatizada de pruebas.
- **Cursor:** Sincronización de búferes de editor y aplicación segura de diffs.
- **Antigravity:** Coordinación del entorno de codificación agéntica y sincronización de tareas.

---

## Verificación y Puertas de Calidad

Todas las contribuciones deben superar estrictamente las puertas de calidad:

```bash
# 1. Puerta estricta Clippy de Rust (cero advertencias, cero errores)
cargo clippy --workspace --all-targets -- -D warnings

# 2. Suite de pruebas completa del espacio de trabajo (>1.670 pruebas, 100% superadas)
cargo test --workspace --all-targets

# 3. Comprobación de tipos y empaquetado de la interfaz de escritorio
cd ui/desktop && npm run build
```

---

## Documentación y Referencias

- **Especificación Arquitectónica Estándar:** [Custos.md](../../Custos.md)
- **Normas para Agentes y Colaboradores:** [AGENTS.md](../../AGENTS.md)
- **Estándares de Diseño de Desktop UI:** [.agents/rules/desktop_ui_architecture_standards.md](../../.agents/rules/desktop_ui_architecture_standards.md)
- **Flujo de Trabajo Git PR:** [.agents/rules/git_pr_workflow.md](../../.agents/rules/git_pr_workflow.md)
- **Índice de Documentación Internacional:** [docs/i18n/README.md](README.md)

---

## Licencia

Custos se distribuye bajo la licencia **GNU AGPL-3.0-or-later**. Incluye una cláusula estricta de Marca Registrada para proteger la identidad de Custos y prohíbe el empaquetado como servicio SaaS comercial de código cerrado. Consulte [LICENSE](../../LICENSE) para más detalles.
