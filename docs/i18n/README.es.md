<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../assets/banner-dark.png">
  <source media="(prefers-color-scheme: light)" srcset="../assets/banner.png">
  <img alt="Custos" src="../assets/banner.png" width="100%">
</picture>

[ EN ](../../README.md) · [ VI ](README.vi.md) · [ DE ](README.de.md) · [ ZH ](README.zh.md) · [ JA ](README.ja.md) · [ KO ](README.ko.md) · [ ES ](README.es.md)

</div>

# Custos SADE

> **Entorno de Desarrollo de Agentes Supervisados (SADE) — Local-First, Basado en Evidencia, Gobernado por Humanos.**

Custos es un entorno de desarrollo y ejecución de prioridad local (local-first) para agentes autónomos en programación (coding), investigación científica (research) y asistencia en flujos de trabajo personales. A diferencia de simples envoltorios de LLM o sistemas multiagente opacos, Custos impone una estricta supervisión humana, persistencia de estado determinista, puertas de capacidad auditadas criptográficamente y la exigencia de pruebas empíricas antes de reconocer cualquier tarea como completada.

---

## Tres Pilares No Negociables

| Pilar | Principio Fundamental | Garantía Arquitectónica |
|---|---|---|
| **P1 — Tarea como Entidad de Primer Nivel** | Las tareas existen independientemente de sesiones y conexiones | Persistencia SQLite WAL duradera, recuperación determinista tras caídas y libro de órdenes idempotente. |
| **P2 — Propuesta ≠ Decisión ≠ Efecto** | El modelo propone; el núcleo autoriza; la pasarela ejecuta | El modelo IA solo emite un `ActionIntent`; el núcleo firma un `ExecutionPermit` de un solo uso; el sandbox aplica las puertas de capacidad. |
| **P3 — Finalización Basada en Evidencia** | Ninguna tarea se completa sin prueba empírica | Las condiciones de aceptación solo se aprueban con artefactos objetivos verificables (diffs, trazas de pruebas, auditorías de citas); cero finalizaciones ficticias. |

---

## 10 Invariantes Inviolables del Sistema

1. **INV-01 (Soberanía Local-First):** Todo el estado, contratos, tareas y registros de telemetría residen exclusivamente en el disco local.
2. **INV-02 (Cero Atomicidad Ilusoria):** Todo cambio de estado se confirma en SQLite antes de despachar acciones externas.
3. **INV-03 (Permiso Previo al Efecto):** Ninguna mutación externa se ejecuta sin un `ExecutionPermit` criptográficamente vinculado.
4. **INV-04 (Vinculación de Identidad del Actor):** La identidad del actor se vincula desde capas de transporte autenticadas, nunca desde cargas autodeclaradas.
5. **INV-05 (Evidencia por Encima de Afirmaciones):** La aceptación de tareas exige artefactos empíricos verificados, no meras confirmaciones del modelo.
6. **INV-06 (Veracidad Explícita en la Interfaz):** La interfaz entra de inmediato en modo de fallo seguro (`unavailable`/`degraded`) si los servicios de backend fallan.
7. **INV-07 (Aislamiento de Arrendamiento de Árbol de Trabajo):** Las escrituras concurrentes en el mismo árbol de trabajo (Worktree) se aíslan para evitar pérdidas de datos.
8. **INV-08 (Límites Estrictos de Recursos):** La ejecución de procesos, el búfer de líneas y la poda de tokens respetan límites estrictos en bytes.
9. **INV-09 (Transportes Auditados):** Toda mutación pasa a través del libro de comandos con verificación de resumen (digest) contra repeticiones.
10. **INV-10 (Supremacía Humana):** El operador humano mantiene la potestad irrestricta de pausar, corregir o abortar tareas en cualquier fase.

---

## Mecanismo de Operación

```mermaid
flowchart LR
    Human[Operador Humano] --> Session[Canal de Sesión / Workbench]
    Session --> Task[Tarea Persistente]
    Task --> Context[Compilación de Contexto]
    Context --> Worker[Modelo / Arnés de Trabajo]
    Worker --> Intent[Intención de Acción (Action Intent)]
    Intent --> Gate[Puerta de Autoridad y Permiso]
    Gate --> Effect[Efecto Aislado en Sandbox]
    Effect --> Evidence[Evidencia Objetiva]
    Evidence --> Outcome[Finalización Verificada]
```

---

## Arquitectura del Sistema: 11 Crates Canónicos en Rust

Custos separa estrictamente la lógica pura de los efectos secundarios en 11 crates organizados por capas:

| Capa | Crate | Responsabilidad Principal |
|---|---|---|
| **Capa 0 (Domain)** | `custos-domain` | Entidades puras del dominio, máquinas de estado e invariantes Zero-I/O. |
| **Capa 1 (Core)** | `custos-core` | Servicio de orquestación de tareas, motor de autoridad y políticas de verificación. |
| | `custos-persistence` | Almacenamiento SQLite WAL, patrón Outbox, transiciones CAS y recuperación tras fallos. |
| | `custos-provider` | Contratos de modelos neutrales, analizador de transmisiones SSE y tokenizadores. |
| **Capa 2 (Runtime & Bridge)** | `custos-bridge` | Promoción de sesión a tarea, enrutamiento RPC y despacho de API local. |
| | `custos-runtime` | Ciclo cognitivo del agente, compresión de contexto (RTK), memoria por niveles y planificadores. |
| | `custos-adapters` | Sandboxes del sistema operativo, clientes MCP, sidecars de inferencia local y traductores. |
| **Capa 3 (Domain Packs)** | `custos-packs` | Flujos de trabajo especializados: Ingeniería de software, investigación y asistencia. |
| **Capa 4 (Entry & Distribution)** | `custos-daemon` | Raíz de composición única (Composition Root), demonio en segundo plano y HTTP/JSON-RPC local. |
| | `custos-sdk` | DTOs de cliente, enlaces UniFFI y clientes de transporte. |
| | `custos-cli` | Interfaz de terminal basada en TUI (`ratatui`). |

---

## Interfaz de Usuario de Escritorio

La interfaz de escritorio de Custos está construida con **Vite, React 19 y Tauri 2**:

- **Jerarquía de 3 Niveles:** Navegación fluida: `Proyecto -> Espacio de Trabajo -> Árbol de Trabajo (Project -> Workspace -> Worktree)`.
- **Sistema de Tokens Graphite JetBlack:** Tema oscuro de alto contraste optimizado para sesiones de trabajo prolongadas.
- **Bancos de Trabajo Dedicados (Workbenches):**
  - **Ingeniería:** Archivos y cambios, visor de evidencias y gestor de árboles de trabajo.
  - **Investigación:** Estudio de síntesis, panel de descubrimientos y libro de ejecuciones.
  - **Asistente y Compartido:** Navegador integrado, terminal, flota remota y notas.
- **Política de Cero Datos Falsos (Zero-Mock Policy):** Todas las métricas de la interfaz reflejan telemetría real del demonio y advierten si un servicio se degrada.

---

## Puertas de Calidad y Pruebas

Toda contribución de código debe superar obligatoriamente las siguientes puertas:

```bash
# 1. Puerta bloqueante Clippy en Rust (0 advertencias, 0 errores)
cargo clippy --workspace --all-targets -- -D warnings

# 2. Suite completa de pruebas (>1.670 pruebas, 100% aprobadas)
cargo test --workspace --all-targets

# 3. Verificación de tipos y compilación de la interfaz de escritorio
cd ui/desktop && npm run build
```

---

## Documentación y Referencias

- **Especificación de Arquitectura:** [Custos.md](../../Custos.md)
- **Reglas para Agentes y Desarrolladores:** [AGENTS.md](../../AGENTS.md)
- **Estándares de Interfaz de Escritorio:** [.agents/rules/desktop_ui_architecture_standards.md](../../.agents/rules/desktop_ui_architecture_standards.md)
- **Flujo de Trabajo Git PR:** [.agents/rules/git_pr_workflow.md](../../.agents/rules/git_pr_workflow.md)
- **Directorio de Documentación:** [docs/README.md](../README.md)

---

## Licencia

Custos se distribuye bajo la licencia **GNU AGPL-3.0-or-later**. Incluye una estricta Cláusula de Marca Registrada para proteger la identidad de Custos y prohíbe el empaquetado comercial SaaS de código cerrado. Consulte [LICENSE](../../LICENSE) para más detalles.
