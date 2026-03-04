use glhf::db::{Database, SearchResult, EMBEDDING_DIM};
use glhf::{ChunkKind, Document};
use std::path::PathBuf;

/// A synthetic corpus for search quality testing.
///
/// Contains ~45 documents across topic clusters designed to test
/// FTS ranking, semantic similarity, and hybrid search behavior.
pub struct SearchCorpus {
    docs: Vec<Document>,
}

impl SearchCorpus {
    /// Builds the standard evaluation corpus with ~45 documents.
    #[allow(clippy::too_many_lines)]
    pub fn standard() -> Self {
        let mut docs = Vec::with_capacity(50);

        // ── Rust cluster (4 docs) ───────────────────────────────────────
        docs.push(
            Document::new(
                ChunkKind::Message,
                "Rust's error handling model uses Result<T, E> and Option<T> types for \
                 representing success and failure. The question mark operator propagates \
                 errors up the call stack automatically. Custom error types built with \
                 thiserror provide domain-specific error variants with descriptive messages. \
                 Always prefer returning Result over panicking in library code. Pattern \
                 matching on error variants lets callers decide how to handle each failure mode."
                    .to_string(),
                PathBuf::from("/corpus/rust/1.jsonl"),
            )
            .with_role(Some("assistant".to_string()))
            .with_session_id(Some("rust-session-001".to_string()))
            .with_project(Some("/Users/dev/rust-project".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "Cargo is Rust's build system and package manager. It manages dependencies \
                 through Cargo.toml, supports workspaces for multi-crate projects, and handles \
                 compilation targets. Use cargo add to include new dependencies and cargo update \
                 to refresh the lock file. Cargo features enable conditional compilation of \
                 optional functionality. The cargo clippy linter catches common mistakes."
                    .to_string(),
                PathBuf::from("/corpus/rust/2.jsonl"),
            )
            .with_role(Some("assistant".to_string()))
            .with_session_id(Some("rust-session-001".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "Testing in Rust uses the built-in test framework. Mark test functions with \
                 #[test] and use assert!, assert_eq!, and assert_ne! macros for verification. \
                 Integration tests live in the tests/ directory and exercise the public API. \
                 Run all tests with cargo test or filter by name. Doc tests in documentation \
                 comments verify examples stay correct as code evolves."
                    .to_string(),
                PathBuf::from("/corpus/rust/3.jsonl"),
            )
            .with_role(Some("user".to_string()))
            .with_session_id(Some("rust-session-002".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "Async Rust relies on the tokio runtime for concurrent I/O operations. Use \
                 async/await syntax with futures to write non-blocking code. The tokio::spawn \
                 function creates new tasks that run concurrently on the thread pool. Select \
                 between multiple futures with tokio::select! macro. Channels provide message \
                 passing between async tasks without shared mutable state."
                    .to_string(),
                PathBuf::from("/corpus/rust/4.jsonl"),
            )
            .with_role(Some("assistant".to_string()))
            .with_session_id(Some("rust-session-002".to_string())),
        );

        // ── Python/ML cluster (3 docs) ─────────────────────────────────
        docs.push(
            Document::new(
                ChunkKind::Message,
                "Python package management uses pip and virtual environments for isolation. \
                 Create isolated environments with python -m venv to avoid dependency conflicts \
                 between projects. Requirements files pin exact versions for reproducible builds. \
                 Use pip install -e for editable development installs that reflect code changes \
                 immediately. Poetry and uv are modern alternatives to pip."
                    .to_string(),
                PathBuf::from("/corpus/python/1.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "NumPy provides efficient n-dimensional arrays for scientific computing in \
                 Python. Matrix operations, linear algebra, and random number generation are \
                 core features. Broadcasting rules allow operations between arrays of different \
                 shapes without explicit loops. NumPy arrays dramatically outperform Python \
                 lists for numerical computation due to contiguous memory layout."
                    .to_string(),
                PathBuf::from("/corpus/python/2.jsonl"),
            )
            .with_role(Some("user".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "Machine learning with scikit-learn involves training models on labeled data \
                 to make predictions. Common algorithms include random forests, support vector \
                 machines, and gradient boosting classifiers. The consistent fit-predict pattern \
                 works across all estimators. Cross-validation helps evaluate model performance \
                 and detect overfitting. Feature scaling with StandardScaler improves convergence."
                    .to_string(),
                PathBuf::from("/corpus/python/3.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        // ── Git cluster (4 docs) ───────────────────────────────────────
        docs.push(
            Document::new(
                ChunkKind::Message,
                "Git push sends local commits to remote repositories for sharing with the team. \
                 Use git push -u origin to set upstream tracking on a new branch. Force push \
                 with --force-with-lease is safer than --force as it checks for concurrent \
                 changes by others. Always communicate with your team before force pushing \
                 shared branches to avoid overwriting their work."
                    .to_string(),
                PathBuf::from("/corpus/git/1.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "Interactive rebase with git rebase -i lets you rewrite commit history before \
                 merging. Squash commits to combine related changes into clean logical units. \
                 Reword commit messages for clarity and consistency. Rebase onto main to keep \
                 feature branches up to date with the latest changes, but never rebase commits \
                 that have already been pushed to a shared remote."
                    .to_string(),
                PathBuf::from("/corpus/git/2.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "Merge conflicts occur when Git cannot automatically reconcile divergent \
                 changes to the same lines. Three-way merge compares the common ancestor with \
                 both branch tips to identify conflicts. Conflict markers (<<<, ===, >>>) show \
                 the competing changes inline. Resolve by choosing one side, combining both \
                 versions, or writing entirely new code that incorporates both intentions."
                    .to_string(),
                PathBuf::from("/corpus/git/3.jsonl"),
            )
            .with_role(Some("user".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "Git status shows the working directory and staging area state at a glance. \
                 Use git diff to see unstaged changes and git diff --staged for changes ready \
                 to commit. The git log command displays commit history with configurable \
                 formatting options like --oneline and --graph. Use git stash to temporarily \
                 shelve changes while switching branches."
                    .to_string(),
                PathBuf::from("/corpus/git/4.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        // ── Database cluster (4 docs) ──────────────────────────────────
        // Doc with "sqlite" mentioned 5 times (for BM25 term frequency test)
        docs.push(
            Document::new(
                ChunkKind::Message,
                "SQLite is a lightweight embedded database engine used everywhere. SQLite \
                 supports FTS5 for full-text search with BM25 ranking algorithms. SQLite \
                 transactions are ACID-compliant even during power failures and crashes. SQLite \
                 databases are single files, making backup and deployment trivially simple. \
                 SQLite is the most widely deployed database engine in the world, found in \
                 phones, browsers, and embedded systems."
                    .to_string(),
                PathBuf::from("/corpus/db/1.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "Connection pooling reduces overhead by reusing established database connections \
                 instead of creating new ones for each request. Libraries like r2d2 and deadpool \
                 manage pools of connections for concurrent access patterns. Configure minimum \
                 and maximum pool sizes based on your expected workload characteristics. \
                 Connection health checks prevent using stale or broken connections that would \
                 cause runtime errors."
                    .to_string(),
                PathBuf::from("/corpus/db/2.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        // Doc with "sqlite" mentioned 1 time (for BM25 comparison)
        docs.push(
            Document::new(
                ChunkKind::Message,
                "Database migrations track schema changes over time in version control. Tools \
                 like sqlx apply migrations for databases including SQLite and PostgreSQL in \
                 order. Each migration has an up and down function for applying and reverting \
                 changes safely. Always test migrations against a copy of production data \
                 before deploying to avoid data loss or corruption."
                    .to_string(),
                PathBuf::from("/corpus/db/3.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "Use the SQL OR operator to combine multiple conditions in WHERE clauses for \
                 flexible queries. OR returns rows matching any of the specified conditions. \
                 Combine OR with AND using parentheses to build complex predicates correctly. \
                 Index columns used in OR conditions to maintain query performance at scale. \
                 UNION provides an alternative to OR for combining separate result sets."
                    .to_string(),
                PathBuf::from("/corpus/db/4.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        // ── Authentication cluster (4 docs, same session) ──────────────
        let auth_session = Some("auth-session-001".to_string());

        docs.push(
            Document::new(
                ChunkKind::Message,
                "JSON Web Tokens provide stateless authentication for web APIs and microservices. \
                 JWTs contain claims like subject, issuer, and expiration time encoded as base64. \
                 Sign tokens with HMAC-SHA256 or RSA keys to prevent tampering. Validate token \
                 signatures and check claims on every incoming request. Refresh tokens extend \
                 session lifetime without requiring the user to re-authenticate."
                    .to_string(),
                PathBuf::from("/corpus/auth/1.jsonl"),
            )
            .with_role(Some("assistant".to_string()))
            .with_session_id(auth_session.clone()),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "The user login flow starts with credential submission over HTTPS. Hash \
                 passwords with bcrypt using a cost factor of at least 12 rounds. Compare \
                 submitted passwords against stored hashes using constant-time comparison. \
                 Rate limit login attempts to prevent brute-force attacks on accounts. Return \
                 consistent generic error messages to avoid leaking whether a username exists."
                    .to_string(),
                PathBuf::from("/corpus/auth/2.jsonl"),
            )
            .with_role(Some("user".to_string()))
            .with_session_id(auth_session.clone()),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "HTTP cookies store session identifiers on the client side between requests. \
                 Set the Secure flag to restrict cookies to HTTPS connections only. The HttpOnly \
                 flag prevents JavaScript access to the cookie, mitigating XSS attacks. The \
                 SameSite attribute controls cross-origin cookie behavior to prevent CSRF. \
                 Session cookies expire automatically when the user closes the browser."
                    .to_string(),
                PathBuf::from("/corpus/auth/3.jsonl"),
            )
            .with_role(Some("assistant".to_string()))
            .with_session_id(auth_session.clone()),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "OAuth2 authorization code flow delegates authentication to an external \
                 identity provider. The client redirects users to the authorization server \
                 for consent. After approval, the server returns an authorization code via \
                 redirect. Exchange the code for access and refresh tokens at the token \
                 endpoint. Use the PKCE extension for public clients like mobile apps and SPAs."
                    .to_string(),
                PathBuf::from("/corpus/auth/4.jsonl"),
            )
            .with_role(Some("assistant".to_string()))
            .with_session_id(auth_session),
        );

        // ── Deployment cluster (3 docs) ────────────────────────────────
        docs.push(
            Document::new(
                ChunkKind::Message,
                "Docker containers package applications with all their dependencies into \
                 portable units. Dockerfiles define the build steps and runtime environment \
                 layer by layer. Multi-stage builds reduce final image size by separating \
                 build tools from runtime artifacts. Use .dockerignore to exclude unnecessary \
                 files and secrets from the build context for security and speed."
                    .to_string(),
                PathBuf::from("/corpus/deploy/1.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "CI/CD pipelines automate building, testing, and deploying code on every \
                 change. GitHub Actions workflows trigger on push or pull request events \
                 automatically. Define jobs with sequential steps that execute shell commands \
                 or reusable actions. Cache dependencies between workflow runs to dramatically \
                 speed up pipeline execution time and reduce costs."
                    .to_string(),
                PathBuf::from("/corpus/deploy/2.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "Deploying code to production requires careful planning with monitoring and \
                 rollback strategies. Blue-green deployments minimize downtime by maintaining \
                 two identical environments and switching traffic. Canary releases gradually \
                 shift a percentage of traffic to the new version. Health checks and alerting \
                 catch regressions before they impact all users in production."
                    .to_string(),
                PathBuf::from("/corpus/deploy/3.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        // ── Frontend/CSS cluster (3 docs) ──────────────────────────────
        docs.push(
            Document::new(
                ChunkKind::Message,
                "CSS flexbox creates one-dimensional layouts for arranging items in rows or \
                 columns. Use justify-content and align-items to control element positioning \
                 along the main and cross axes. Flex-grow and flex-shrink determine how child \
                 items share available space. Flexbox simplifies the historically difficult \
                 task of centering elements both horizontally and vertically."
                    .to_string(),
                PathBuf::from("/corpus/frontend/1.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "CSS grid provides powerful two-dimensional layout control with explicit rows \
                 and columns. Define grid tracks with grid-template-rows and grid-template-columns \
                 properties. Place items precisely with grid-column and grid-row positioning. \
                 Responsive layouts use minmax() and auto-fill to create fluid grids that \
                 adapt to screen size without requiring media queries."
                    .to_string(),
                PathBuf::from("/corpus/frontend/2.jsonl"),
            )
            .with_role(Some("user".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "JavaScript DOM manipulation updates the webpage content dynamically without \
                 full page reloads. Use querySelector and getElementById to select specific \
                 elements. Event listeners respond to user interactions like clicks, hover, and \
                 keyboard input. The fetch API makes asynchronous HTTP requests to load data \
                 from servers and update the page content in response."
                    .to_string(),
                PathBuf::from("/corpus/frontend/3.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        // ── Error tool_results (4 docs) ────────────────────────────────
        docs.push(
            Document::new(
                ChunkKind::ToolResult,
                "ENOENT: no such file or directory, open '/Users/dev/project/config.yaml'. \
                 The file was expected at this path but does not exist. Check that the path \
                 is spelled correctly and the file has not been moved or deleted by another \
                 process. Also verify the working directory is correct."
                    .to_string(),
                PathBuf::from("/corpus/errors/1.jsonl"),
            )
            .with_is_error(Some(true))
            .with_tool_name(Some("Bash".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::ToolResult,
                "Segmentation fault (core dumped) at address 0x0000000000000000. Signal 11 \
                 (SIGSEGV) received in thread main. The program attempted to dereference a \
                 null pointer or access memory outside its allocated region. Check for null \
                 pointer dereferences, buffer overflows, or use-after-free bugs in native code."
                    .to_string(),
                PathBuf::from("/corpus/errors/2.jsonl"),
            )
            .with_is_error(Some(true))
            .with_tool_name(Some("Bash".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::ToolResult,
                "TypeError: Cannot read property 'length' of undefined at processItems \
                 (app.js:42:15). The variable was not initialized before accessing its \
                 properties. Add a null check or use optional chaining (?.) to handle \
                 missing values gracefully. Review the call site to ensure the argument \
                 is passed correctly."
                    .to_string(),
                PathBuf::from("/corpus/errors/3.jsonl"),
            )
            .with_is_error(Some(true))
            .with_tool_name(Some("Bash".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::ToolResult,
                "Broken pipe error when writing to stdout: connection reset by peer. The \
                 receiving end of the pipe was closed before all data could be written. This \
                 commonly occurs when piping output to head or a process that exits early. \
                 To fix this, handle SIGPIPE signals or check if the reader is still alive \
                 before each write operation."
                    .to_string(),
                PathBuf::from("/corpus/errors/4.jsonl"),
            )
            .with_is_error(Some(true))
            .with_tool_name(Some("Bash".to_string())),
        );

        // ── Tool invocations (4 docs) ──────────────────────────────────
        docs.push(
            Document::new(
                ChunkKind::ToolUse,
                "git status".to_string(),
                PathBuf::from("/corpus/tools/1.jsonl"),
            )
            .with_tool_name(Some("Bash".to_string()))
            .with_tool_id(Some("tool-001".to_string()))
            .with_tool_input(Some(r#"{"command":"git status"}"#.to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::ToolUse,
                "cargo test --release".to_string(),
                PathBuf::from("/corpus/tools/2.jsonl"),
            )
            .with_tool_name(Some("Bash".to_string()))
            .with_tool_id(Some("tool-002".to_string()))
            .with_tool_input(Some(r#"{"command":"cargo test --release"}"#.to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::ToolUse,
                "/src/main.rs".to_string(),
                PathBuf::from("/corpus/tools/3.jsonl"),
            )
            .with_tool_name(Some("Read".to_string()))
            .with_tool_id(Some("tool-003".to_string()))
            .with_tool_input(Some(r#"{"file_path":"/src/main.rs"}"#.to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::ToolUse,
                "fn process_items(items: &[Item]) -> Result<Vec<Output>>".to_string(),
                PathBuf::from("/corpus/tools/4.jsonl"),
            )
            .with_tool_name(Some("Edit".to_string()))
            .with_tool_id(Some("tool-004".to_string()))
            .with_tool_input(Some(
                r#"{"file_path":"/src/lib.rs","old_string":"fn process_items"}"#.to_string(),
            )),
        );

        // ── Special character docs (2 docs) ────────────────────────────
        docs.push(
            Document::new(
                ChunkKind::Message,
                "C++ templates enable generic programming with compile-time type safety and \
                 zero-cost abstractions. Template specialization handles specific types with \
                 optimized implementations. Modern C++ standards from C++11 onwards introduced \
                 auto type deduction, lambda expressions, move semantics, and smart pointers \
                 like unique_ptr and shared_ptr for memory safety."
                    .to_string(),
                PathBuf::from("/corpus/special/1.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "Configure the $PATH environment variable to include custom binary directories \
                 for your shell. Add export PATH=$HOME/.local/bin:$PATH to your shell profile \
                 file. The system searches directories listed in $PATH from left to right to \
                 find executables. Use which or command -v to verify which binary $PATH resolves."
                    .to_string(),
                PathBuf::from("/corpus/special/2.jsonl"),
            )
            .with_role(Some("user".to_string())),
        );

        // ── Near-miss distractors (8 docs) ─────────────────────────────
        // These share vocabulary with target clusters but are semantically different.
        // A good model should distinguish; a bag-of-words model will struggle.

        // Shares "error handling" vocabulary but is Python-specific
        docs.push(
            Document::new(
                ChunkKind::Message,
                "Python exception handling uses try/except blocks to catch and recover from \
                 runtime errors. The traceback module provides detailed stack traces for \
                 debugging. Raise custom exceptions by subclassing Exception. Use finally \
                 blocks for cleanup actions that must run regardless of whether an error \
                 occurred. Context managers with the with statement handle resource cleanup \
                 automatically."
                    .to_string(),
                PathBuf::from("/corpus/distractor/1.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        // Keyword collision: "Rust" the game, not the language
        docs.push(
            Document::new(
                ChunkKind::Message,
                "Rust is a multiplayer survival video game where players gather resources \
                 like wood and stone to build bases and craft weapons. The game features \
                 a harsh environment with wildlife, radiation zones, and other players as \
                 threats. Rust uses a progression system where blueprints unlock advanced \
                 items. Server wipes reset all player progress periodically to keep the \
                 game fresh and competitive."
                    .to_string(),
                PathBuf::from("/corpus/distractor/2.jsonl"),
            )
            .with_role(Some("user".to_string())),
        );

        // "Pipeline" in data engineering, not CI/CD
        docs.push(
            Document::new(
                ChunkKind::Message,
                "Apache Airflow orchestrates complex data pipelines using directed acyclic \
                 graphs. Each DAG defines tasks and their dependencies for ETL workflows. \
                 Operators execute specific actions like running SQL queries, transferring \
                 files, or calling APIs. The scheduler triggers DAG runs based on time \
                 intervals or external events. Pipeline failures send alerts through \
                 configurable notification channels."
                    .to_string(),
                PathBuf::from("/corpus/distractor/3.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        // "Branches" in ML/decision trees, not git
        docs.push(
            Document::new(
                ChunkKind::Message,
                "Decision tree branches split training data based on feature thresholds \
                 that maximize information gain. Each internal node tests a condition, and \
                 leaf nodes contain class predictions or regression values. Tree depth and \
                 minimum samples per branch control overfitting. Random forests aggregate \
                 predictions from many independently trained decision trees to reduce variance \
                 and improve generalization."
                    .to_string(),
                PathBuf::from("/corpus/distractor/4.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        // "Container" in CSS, not Docker
        docs.push(
            Document::new(
                ChunkKind::Message,
                "CSS container queries let components adapt their styling based on their \
                 parent container's size rather than the viewport. Define containment \
                 contexts with container-type: inline-size. Write @container rules that \
                 apply styles when the container meets size conditions. This enables truly \
                 reusable components that look correct regardless of where they're placed \
                 in the layout hierarchy."
                    .to_string(),
                PathBuf::from("/corpus/distractor/5.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        // "Merge" in pandas/data, not git
        docs.push(
            Document::new(
                ChunkKind::Message,
                "Pandas merge and join operations combine DataFrames on shared columns or \
                 indices. Inner merge keeps only matching rows, while outer merge preserves \
                 all rows with NaN fill for missing values. The on parameter specifies join \
                 keys, and suffixes disambiguate overlapping column names. For time-series, \
                 merge_asof joins on nearest key values within a tolerance window."
                    .to_string(),
                PathBuf::from("/corpus/distractor/6.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        // "Tokens" in rate limiting, not JWT
        docs.push(
            Document::new(
                ChunkKind::Message,
                "Token bucket rate limiting controls API request throughput by maintaining \
                 a bucket of tokens that refills at a fixed rate. Each request consumes one \
                 token; requests are rejected when the bucket is empty. Configure burst size \
                 and refill rate based on your API's capacity. Sliding window counters provide \
                 smoother rate limiting than fixed windows. Redis-based implementations work \
                 across distributed systems."
                    .to_string(),
                PathBuf::from("/corpus/distractor/7.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        // Low-level memory management (not Rust ownership/borrowing)
        docs.push(
            Document::new(
                ChunkKind::Message,
                "Assembly language memory management requires manual allocation on the heap \
                 using system calls like brk or mmap. The stack pointer register tracks the \
                 current stack frame for local variables. Memory-mapped I/O lets hardware \
                 devices appear as memory addresses. Virtual memory pages are mapped by the \
                 MMU, and page faults trigger the OS to load data from disk into physical RAM."
                    .to_string(),
                PathBuf::from("/corpus/distractor/8.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        // ── Technically adjacent docs (11 docs) ────────────────────────
        // These are topically nearby to target clusters, increasing competition.

        docs.push(
            Document::new(
                ChunkKind::Message,
                "Securing REST API endpoints requires authentication middleware that \
                 validates credentials before allowing access to protected resources. \
                 Implement CORS policies to control which origins can make requests. \
                 Input validation prevents injection attacks on query parameters and \
                 request bodies. API versioning through URL paths or headers manages \
                 backward compatibility as endpoints evolve."
                    .to_string(),
                PathBuf::from("/corpus/adjacent/1.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "Load balancing distributes incoming network traffic across multiple backend \
                 servers to ensure high availability. Round-robin, least-connections, and \
                 IP-hash are common algorithms. Health checks automatically remove unhealthy \
                 servers from the pool. Layer 7 load balancers can route based on HTTP headers, \
                 paths, or cookies. Horizontal scaling adds more servers behind the balancer."
                    .to_string(),
                PathBuf::from("/corpus/adjacent/2.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "Redis serves as an in-memory key-value store commonly used for caching \
                 session data and reducing database load. Expiration policies automatically \
                 evict stale entries. Redis pub/sub enables real-time messaging between \
                 services. Persistence options include RDB snapshots and AOF logging. Redis \
                 Cluster provides horizontal partitioning across multiple nodes for scaling."
                    .to_string(),
                PathBuf::from("/corpus/adjacent/3.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "Kubernetes orchestrates containerized applications across clusters of \
                 machines. Pods are the smallest deployable units containing one or more \
                 containers. Deployments manage rollout strategies like rolling updates \
                 and recreate. Services provide stable network endpoints for pod groups. \
                 ConfigMaps and Secrets inject configuration without rebuilding images."
                    .to_string(),
                PathBuf::from("/corpus/adjacent/4.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "Code review best practices include keeping pull requests small and focused \
                 on a single concern. Write clear PR descriptions explaining the motivation \
                 and approach. Review for correctness, readability, and edge cases rather than \
                 style preferences. Use automated linters and formatters to eliminate style \
                 debates. Respond to review feedback promptly and respectfully."
                    .to_string(),
                PathBuf::from("/corpus/adjacent/5.jsonl"),
            )
            .with_role(Some("user".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "Terraform infrastructure as code provisions and manages cloud resources \
                 declaratively. HCL configuration files describe the desired state of \
                 infrastructure. The plan command previews changes before applying them. \
                 State files track resource mappings between config and real infrastructure. \
                 Modules encapsulate reusable patterns for VPCs, databases, and compute."
                    .to_string(),
                PathBuf::from("/corpus/adjacent/6.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "GraphQL schemas define types, queries, and mutations for a strongly-typed \
                 API. Clients request exactly the fields they need, avoiding over-fetching. \
                 Resolvers map schema fields to data sources. Subscriptions enable real-time \
                 updates pushed to clients over WebSocket connections. Schema stitching \
                 combines multiple GraphQL services into a unified gateway."
                    .to_string(),
                PathBuf::from("/corpus/adjacent/7.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "WebSocket connections provide full-duplex bidirectional communication between \
                 client and server. The initial HTTP upgrade handshake establishes a persistent \
                 connection. Messages flow in both directions without polling. Socket.io adds \
                 reconnection, rooms, and namespaces on top of raw WebSockets. Use ping/pong \
                 frames to detect dropped connections."
                    .to_string(),
                PathBuf::from("/corpus/adjacent/8.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "TLS certificates establish encrypted HTTPS connections between browsers and \
                 servers. Certificate authorities validate domain ownership before issuing \
                 certificates. Let's Encrypt provides free automated certificates. The TLS \
                 handshake negotiates cipher suites and exchanges keys. Certificate pinning \
                 prevents man-in-the-middle attacks by restricting trusted certificates."
                    .to_string(),
                PathBuf::from("/corpus/adjacent/9.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "Regular expressions match patterns in text using a concise notation. \
                 Character classes like \\d and \\w match digits and word characters. \
                 Quantifiers control repetition: * for zero or more, + for one or more. \
                 Lookahead and lookbehind assertions match positions without consuming \
                 characters. Named capture groups extract matched substrings for further \
                 processing in code."
                    .to_string(),
                PathBuf::from("/corpus/adjacent/10.jsonl"),
            )
            .with_role(Some("user".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "Microservices architecture decomposes applications into independently \
                 deployable services communicating via APIs or message queues. Each service \
                 owns its data and can be developed, deployed, and scaled separately. Service \
                 discovery registers and locates instances dynamically. Circuit breakers \
                 prevent cascading failures when downstream services are unavailable. \
                 Distributed tracing tracks requests across service boundaries."
                    .to_string(),
                PathBuf::from("/corpus/adjacent/11.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        // ── Homonym/polysemy pairs (12 docs, 6 pairs) ─────────────────
        // Each pair shares a keyword but has completely different meaning.
        // Good models disambiguate via context; bag-of-words models fail.

        // "thread" — concurrency vs forum discussion
        docs.push(
            Document::new(
                ChunkKind::Message,
                "Thread pools manage concurrent execution of tasks across CPU cores. Spawn \
                 worker threads to parallelize compute-heavy operations without blocking the \
                 main thread. Mutex and RwLock synchronize shared mutable state between threads \
                 safely. Thread-local storage gives each thread its own independent copy of \
                 data. Rayon's par_iter automatically distributes iterator work across the \
                 thread pool with work-stealing scheduling."
                    .to_string(),
                PathBuf::from("/corpus/homonym/thread_tech.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "The discussion thread on the community forum accumulated over two hundred \
                 replies from developers sharing their experiences and opinions. Thread \
                 participants debated the merits of different architectural approaches at \
                 length. Starting a new thread is recommended when the conversation diverges \
                 significantly from the original post topic. Moderators lock threads that \
                 become unproductive or hostile."
                    .to_string(),
                PathBuf::from("/corpus/homonym/thread_forum.jsonl"),
            )
            .with_role(Some("user".to_string())),
        );

        // "stack" — programming vs physical objects
        docs.push(
            Document::new(
                ChunkKind::Message,
                "A stack overflow occurs when recursive function calls exhaust the available \
                 call stack memory. Each function invocation pushes a new frame onto the call \
                 stack containing local variables and the return address. Tail call optimization \
                 prevents stack overflow by reusing the current frame for recursive calls. \
                 Increase the default stack size with ulimit -s or thread builder settings \
                 for deeply recursive algorithms."
                    .to_string(),
                PathBuf::from("/corpus/homonym/stack_tech.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "The stack of unread papers on my desk keeps growing taller every week despite \
                 my best efforts. I need to sort through the entire stack and file the important \
                 documents before they get lost. The bottom of the stack has items from three \
                 months ago that I still have not reviewed. Organizing papers into labeled \
                 folders would prevent this stack from accumulating again."
                    .to_string(),
                PathBuf::from("/corpus/homonym/stack_papers.jsonl"),
            )
            .with_role(Some("user".to_string())),
        );

        // "port" — networking vs harbor
        docs.push(
            Document::new(
                ChunkKind::Message,
                "Network ports identify specific services running on a host machine using \
                 16-bit numbers. HTTP servers typically listen on port 80 or port 443 for \
                 TLS encrypted traffic. Use netstat or ss to check which ports are currently \
                 in use. Firewall rules control access by allowing or blocking traffic on \
                 specific port ranges. Port forwarding through NAT maps external ports to \
                 internal services on a private network."
                    .to_string(),
                PathBuf::from("/corpus/homonym/port_network.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "The shipping port handles thousands of cargo containers arriving by sea \
                 each day from international trade routes. Port authorities coordinate vessel \
                 traffic patterns and berth assignments for efficient throughput. The harbor \
                 master schedules loading and unloading operations to minimize wait times for \
                 incoming cargo ships. Dredging maintains sufficient water depth for large \
                 container vessels to navigate the port channel safely."
                    .to_string(),
                PathBuf::from("/corpus/homonym/port_harbor.jsonl"),
            )
            .with_role(Some("user".to_string())),
        );

        // "log" — application logging vs timber
        docs.push(
            Document::new(
                ChunkKind::Message,
                "Structured logging with JSON format enables efficient log aggregation and \
                 full-text search across services. Configure log levels (debug, info, warn, \
                 error) to control output verbosity per module. Centralized log collection \
                 with Fluentd or Vector ships logs to Elasticsearch for analysis and alerting. \
                 Include correlation IDs in every log entry to trace individual requests across \
                 distributed service boundaries."
                    .to_string(),
                PathBuf::from("/corpus/homonym/log_tech.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "The fallen log across the hiking trail was covered in thick green moss and \
                 clusters of wild mushrooms. Loggers historically floated timber downstream \
                 to the sawmill during spring when river levels were highest. The old cabin \
                 was built from hand-hewn logs carefully stacked and notched at the corners \
                 for structural stability. A hollow log near the stream provided shelter for \
                 small woodland creatures during winter storms."
                    .to_string(),
                PathBuf::from("/corpus/homonym/log_timber.jsonl"),
            )
            .with_role(Some("user".to_string())),
        );

        // "class" — OOP vs education
        docs.push(
            Document::new(
                ChunkKind::Message,
                "Object-oriented class hierarchies define inheritance relationships between \
                 types in languages like Python and Java. Abstract base classes declare \
                 interfaces that concrete subclasses must implement. Multiple inheritance \
                 uses method resolution order to handle the diamond problem. Composition over \
                 inheritance is often preferred for flexibility and testability. Mixins add \
                 reusable behavior to classes without creating deep hierarchies."
                    .to_string(),
                PathBuf::from("/corpus/homonym/class_oop.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "The university class on medieval European history meets every Tuesday and \
                 Thursday afternoon in the lecture hall. Professor Williams assigns weekly \
                 reading chapters from the textbook and expects a five-page analytical essay \
                 at the end of each semester. Class participation counts for fifteen percent \
                 of the final grade. Students who miss more than three classes without prior \
                 arrangement may be administratively dropped from the roster."
                    .to_string(),
                PathBuf::from("/corpus/homonym/class_school.jsonl"),
            )
            .with_role(Some("user".to_string())),
        );

        // "shell" — scripting vs marine biology
        docs.push(
            Document::new(
                ChunkKind::Message,
                "Shell scripting automates repetitive system tasks using bash, zsh, or fish \
                 interpreters. Pipe commands together with the | operator to build composable \
                 data processing pipelines. Environment variables configure program behavior \
                 without modifying source code. Shell scripts use exit codes to signal success \
                 or failure to calling processes. Shebang lines like #!/bin/bash specify which \
                 interpreter executes the script file."
                    .to_string(),
                PathBuf::from("/corpus/homonym/shell_bash.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "The nautilus shell exhibits a perfect logarithmic spiral pattern that \
                 mathematicians have studied for centuries. Marine biologists research shell \
                 formation to understand how mollusks crystallize calcium carbonate into \
                 intricate protective structures. Shells shield soft-bodied creatures from \
                 predators and harsh ocean conditions. Beach combers collect shells of various \
                 shapes, sizes, and iridescent colors along the shoreline after major storms."
                    .to_string(),
                PathBuf::from("/corpus/homonym/shell_marine.jsonl"),
            )
            .with_role(Some("user".to_string())),
        );

        // ── More technical depth (8 docs) ──────────────────────────────

        docs.push(
            Document::new(
                ChunkKind::Message,
                "Application monitoring with Prometheus collects time-series metrics from \
                 instrumented services using a pull-based scraping model. Grafana dashboards \
                 visualize CPU usage, request latency percentiles, and error rates in real \
                 time. Alert rules trigger PagerDuty notifications when metrics exceed defined \
                 thresholds. The RED method tracks request Rate, Errors, and Duration as the \
                 core service-level indicators. SLOs define acceptable performance targets \
                 with error budgets."
                    .to_string(),
                PathBuf::from("/corpus/depth/1.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "Message queues decouple producers from consumers in distributed systems for \
                 reliable asynchronous communication. RabbitMQ routes messages through exchanges \
                 to queues based on binding keys and routing patterns. Apache Kafka uses \
                 partitioned append-only logs for high-throughput event streaming with consumer \
                 groups. Dead letter queues capture messages that fail processing after repeated \
                 retry attempts. Backpressure mechanisms prevent queue overflow during sudden \
                 traffic spikes."
                    .to_string(),
                PathBuf::from("/corpus/depth/2.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "Database indexing accelerates query performance by creating sorted lookup \
                 structures alongside table data. B-tree indexes handle both range queries \
                 and equality comparisons efficiently with logarithmic lookup time. Composite \
                 indexes cover multiple columns in a specific order matching query patterns. \
                 Covering indexes include all queried columns to avoid expensive table lookups \
                 entirely. Over-indexing degrades write performance and increases storage costs."
                    .to_string(),
                PathBuf::from("/corpus/depth/3.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "Memory profiling detects leaks and excessive allocation in running programs. \
                 Valgrind memcheck identifies uninitialized memory reads and leaked allocations \
                 in C and C++ programs. Heap snapshots compare object retention between time \
                 points to find accumulating references. Flame graphs visualize CPU time and \
                 memory allocation hot spots in call trees. Weak references allow garbage \
                 collection of objects that only have weak incoming references."
                    .to_string(),
                PathBuf::from("/corpus/depth/4.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "Property-based testing generates random inputs to verify that program \
                 invariants hold across thousands of cases automatically. QuickCheck-style \
                 libraries shrink failing inputs to find the minimal reproduction case. \
                 Stateful property tests model system behavior as state machines with \
                 transitions and postconditions. Hypothesis for Python and proptest for Rust \
                 are widely used implementations. Property tests complement traditional \
                 example-based unit tests by exploring edge cases."
                    .to_string(),
                PathBuf::from("/corpus/depth/5.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "The observer design pattern notifies registered dependent objects whenever \
                 a subject's internal state changes. Event emitters in Node.js implement \
                 this pattern for loosely coupled component communication. Reactive programming \
                 extends the observer concept with composable streams and transformation \
                 operators like map, filter, and reduce. The publish-subscribe variant scales \
                 to distributed systems through external message brokers like Redis or NATS."
                    .to_string(),
                PathBuf::from("/corpus/depth/6.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "HTTP/2 multiplexes multiple request-response streams over a single TCP \
                 connection eliminating head-of-line blocking at the HTTP layer. Server push \
                 proactively sends resources that the client will likely need next. HPACK \
                 header compression reduces overhead for repeated headers across requests. \
                 Stream prioritization lets clients indicate which resources are most important. \
                 HTTP/3 replaces TCP with QUIC over UDP to eliminate transport-layer \
                 head-of-line blocking entirely."
                    .to_string(),
                PathBuf::from("/corpus/depth/7.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        docs.push(
            Document::new(
                ChunkKind::Message,
                "Compiler front-ends parse source code into abstract syntax trees through \
                 lexing and parsing phases that validate grammar rules. Type checking traverses \
                 the AST to validate expression types and detect errors before code generation. \
                 LLVM intermediate representation enables powerful optimizations independent of \
                 both the source language and target architecture. Register allocation maps \
                 virtual registers to physical CPU registers using graph coloring algorithms. \
                 Link-time optimization enables cross-module function inlining."
                    .to_string(),
                PathBuf::from("/corpus/depth/8.jsonl"),
            )
            .with_role(Some("assistant".to_string())),
        );

        // ── Workplace/noise filler (26 docs) ───────────────────────────
        let filler = [
            "Had a great lunch meeting today to discuss upcoming project timelines and milestones.",
            "The weather forecast shows sunny skies with mild temperatures for the rest of the week.",
            "Remember to water the office plants every Monday and Thursday without fail.",
            "Updated my phone settings to enable dark mode across all installed applications.",
            "Scheduled a dentist appointment for next Tuesday afternoon at three o'clock.",
            "The parking lot on the north side of the building has much better shade coverage.",
            "Picked up groceries on the way home from the office yesterday evening.",
            "The new coffee machine in the break room makes surprisingly good espresso drinks.",
            "Need to renew my driver's license at the DMV before it expires next month.",
            "The team building event is scheduled for the last Friday of this month at the park.",
            "Sprint planning took longer than expected because we underestimated the backlog items.",
            "The candidate interview went well but we need to check references before making an offer.",
            "Writing a technical blog post about the lessons learned from our recent outage.",
            "Setting up the new development laptop with homebrew and all the required toolchains.",
            "Pair programming session was productive but exhausting after three hours straight.",
            "Technical debt in the notification system is slowing down feature development velocity.",
            "Opened a discussion about contributing our utility library to the open source community.",
            "The remote standup worked better today with the new video conferencing tool.",
            "Bought a new ergonomic keyboard with split layout and tactile mechanical switches.",
            "The quarterly all-hands meeting covered company strategy and upcoming product launches.",
            "Fixed the squeaky door hinge in the conference room with some lubricant spray.",
            "The vending machine on the third floor is out of sparkling water again this week.",
            "Rearranged the desk setup to reduce the monitor glare coming from the window.",
            "The fire drill evacuation went smoothly and everyone assembled at the designated point.",
            "Ordered new business cards with the updated company logo and contact information.",
            "The air conditioning unit in the server room needs maintenance before summer heat.",
        ];

        for (i, text) in filler.iter().enumerate() {
            docs.push(
                Document::new(
                    ChunkKind::Message,
                    (*text).to_string(),
                    PathBuf::from(format!("/corpus/filler/{i}.jsonl")),
                )
                .with_role(Some("user".to_string())),
            );
        }

        // ════════════════════════════════════════════════════════════════
        // Bulk corpus (~400 more docs to reach ~500 total)
        // Uses helper closures and arrays for concise declaration.
        // ════════════════════════════════════════════════════════════════

        let mut bulk_id = 0_usize;
        let mut push_msg = |docs: &mut Vec<Document>, text: &str, prefix: &str| {
            docs.push(
                Document::new(
                    ChunkKind::Message,
                    text.to_string(),
                    PathBuf::from(format!("{prefix}/{bulk_id}.jsonl")),
                )
                .with_role(Some(
                    if bulk_id % 3 == 0 {
                        "user"
                    } else {
                        "assistant"
                    }
                    .to_string(),
                )),
            );
            bulk_id += 1;
        };

        // ── Programming languages (25 docs) ────────────────────────────

        for text in [
            "Go goroutines are lightweight threads managed by the Go runtime. Channels \
             provide typed conduits for sending values between goroutines safely. The select \
             statement multiplexes channel operations.",
            "Java Spring Boot auto-configures web applications with embedded Tomcat. Dependency \
             injection wires beans together. Spring Data JPA simplifies database access with \
             repository interfaces.",
            "TypeScript adds static type checking to JavaScript. Union types, generics, and \
             mapped types enable precise type definitions. The compiler catches errors at \
             build time rather than runtime.",
            "Swift optionals explicitly represent the absence of a value. Optional chaining \
             with ?. safely navigates nested optional properties. Guard statements provide \
             early returns when unwrapping fails.",
            "Kotlin coroutines simplify asynchronous programming on the JVM. Structured \
             concurrency ensures child coroutines complete before parents. Flow provides \
             cold asynchronous data streams with backpressure.",
            "Elixir processes are lightweight actors supervised by OTP supervisors. GenServer \
             handles synchronous and asynchronous messages. The BEAM VM enables fault-tolerant \
             distributed systems.",
            "Haskell's type system enforces purity through monads for side effects. Type \
             classes define shared interfaces across types. Lazy evaluation defers computation \
             until values are actually needed.",
            "Ruby blocks, procs, and lambdas are closures with different behaviors. \
             Metaprogramming with define_method and method_missing enables dynamic APIs. \
             Rails conventions reduce configuration boilerplate.",
            "PHP Laravel provides an expressive ORM called Eloquent for database operations. \
             Blade templates compile to cached PHP for fast rendering. Artisan commands \
             scaffold code and run migrations.",
            "Scala combines object-oriented and functional programming on the JVM. Case classes \
             provide immutable data containers with pattern matching. Akka actors handle \
             concurrent message processing.",
            "Go interfaces are satisfied implicitly without explicit declarations. Any type \
             that implements all methods satisfies the interface. The io.Reader interface \
             unifies file, network, and buffer reading.",
            "Java garbage collection pauses can impact latency-sensitive applications. G1 \
             collector divides the heap into regions for incremental collection. ZGC targets \
             sub-millisecond pause times for large heaps.",
            "TypeScript decorators add metadata and behavior to classes and methods at design \
             time. Reflect.metadata stores type information for dependency injection frameworks. \
             Experimental decorator support requires tsconfig flags.",
            "Swift SwiftUI uses declarative syntax to compose views from small reusable \
             components. Property wrappers like @State and @Binding manage view state. \
             Previews render live updates in Xcode as code changes.",
            "Kotlin sealed classes restrict inheritance to a closed set of subclasses for \
             exhaustive when expressions. Data classes auto-generate equals, hashCode, and \
             copy methods. Extension functions add behavior without modifying original classes.",
            "Python asyncio event loop runs coroutines concurrently on a single thread. \
             async/await syntax makes asynchronous code read like synchronous code. \
             aiohttp and httpx provide async HTTP client libraries.",
            "Zig comptime evaluates expressions at compile time for zero-cost abstractions. \
             Manual memory management uses allocators explicitly. No hidden control flow or \
             hidden allocations unlike C.",
            "OCaml algebraic data types combine variants and records for precise domain \
             modeling. Pattern matching exhaustively handles all cases. The module system \
             with functors provides parameterized abstraction.",
            "Clojure persistent data structures share structure between versions for efficient \
             immutable updates. Atoms provide thread-safe mutable references. REPL-driven \
             development enables interactive exploration.",
            "Lua's lightweight embedding makes it popular for game scripting and configuration. \
             Tables serve as arrays, dictionaries, and objects. Metatables customize operator \
             behavior and inheritance.",
            "Perl regular expressions are deeply integrated into the language syntax. One-liners \
             process text files with -pe and -ne flags. CPAN provides thousands of community \
             modules.",
            "R data frames organize tabular data with named columns and row indices. The \
             tidyverse packages provide consistent verbs for data manipulation. ggplot2 \
             creates publication-quality visualizations.",
            "Julia achieves C-like performance through JIT compilation with LLVM. Multiple \
             dispatch selects method implementations based on all argument types. Built-in \
             support for distributed and GPU computing.",
            "Dart and Flutter build cross-platform mobile apps from a single codebase. \
             Widgets compose into a declarative UI tree. Hot reload applies code changes \
             instantly during development.",
            "WebAssembly runs compiled code at near-native speed in web browsers. WASI \
             extends WebAssembly to run outside browsers with system interface access. \
             Languages like Rust, C, and Go compile to WASM modules.",
        ] {
            push_msg(&mut docs, text, "/corpus/bulk/lang");
        }

        // ── Networking & protocols (20 docs) ───────────────────────────

        for text in [
            "TCP three-way handshake establishes reliable connections with SYN, SYN-ACK, ACK \
             packets. Sequence numbers track data order. Retransmission handles lost segments.",
            "UDP provides connectionless datagram delivery without guaranteed ordering or \
             reliability. Lower overhead than TCP makes it suitable for real-time audio, \
             video, and gaming.",
            "HTTP GET retrieves resources idempotently while POST submits data for processing. \
             PUT replaces resources entirely and PATCH applies partial updates. DELETE removes \
             resources from the server.",
            "REST API design uses nouns for resource URLs and HTTP methods for operations. \
             HATEOAS includes links in responses for API discoverability. Pagination with \
             cursor tokens handles large result sets.",
            "gRPC uses Protocol Buffers for strongly-typed service definitions and binary \
             serialization. Bidirectional streaming enables real-time communication. Code \
             generation produces client and server stubs automatically.",
            "WebRTC enables peer-to-peer audio and video communication in browsers without \
             plugins. ICE candidates negotiate NAT traversal paths. STUN and TURN servers \
             relay traffic when direct connections fail.",
            "SSH tunneling encrypts traffic through a secure channel for remote access. Port \
             forwarding maps local ports to remote services. Public key authentication \
             eliminates password-based login risks.",
            "SMTP delivers email between mail servers using a store-and-forward model. SPF \
             and DKIM records authenticate sender identity. DMARC policies specify how to \
             handle authentication failures.",
            "DNS A records map domain names to IPv4 addresses. CNAME records create aliases \
             pointing to canonical names. MX records specify mail servers. TTL controls \
             how long resolvers cache records.",
            "DHCP automatically assigns IP addresses, subnet masks, and default gateways \
             to network devices. Lease times control address reuse. DHCP relay agents \
             forward requests across subnets.",
            "ARP resolves IPv4 addresses to MAC addresses on local network segments. ARP \
             spoofing attacks redirect traffic by sending false mappings. Static ARP \
             entries prevent spoofing on critical systems.",
            "ICMP echo requests and replies implement the ping utility for connectivity \
             testing. Traceroute uses incrementing TTL values to discover network path \
             hops. MTU discovery prevents packet fragmentation.",
            "BGP routing exchanges reachability information between autonomous systems on the \
             internet. Route policies filter and prioritize paths. BGP hijacking diverts \
             traffic through unauthorized networks.",
            "VPN protocols like WireGuard and OpenVPN create encrypted tunnels over public \
             networks. Split tunneling routes only specific traffic through the VPN. \
             Full-tunnel mode encrypts all traffic.",
            "Reverse proxy servers like nginx and HAProxy terminate client connections and \
             forward requests to backend servers. SSL termination offloads encryption. \
             Request buffering protects backends from slow clients.",
            "CDN edge servers cache static content closer to end users geographically. Cache \
             invalidation propagates updates across edge locations. Origin shielding reduces \
             load on the source server.",
            "NAT translates private IP addresses to public addresses for internet access. \
             Port address translation maps multiple internal hosts through a single public \
             IP. NAT traversal techniques enable peer-to-peer connections.",
            "Mutual TLS requires both client and server to present certificates during the \
             handshake. Certificate pinning restricts accepted certificates. mTLS secures \
             service-to-service communication in microservices.",
            "IPv6 uses 128-bit addresses with eight hexadecimal groups separated by colons. \
             Stateless address autoconfiguration eliminates DHCP dependency. Dual-stack \
             runs IPv4 and IPv6 simultaneously during transition.",
            "QUIC protocol combines transport and encryption in a single handshake over UDP. \
             Connection migration maintains sessions across network changes. Independent \
             stream multiplexing avoids head-of-line blocking.",
        ] {
            push_msg(&mut docs, text, "/corpus/bulk/net");
        }

        // ── Security (20 docs) ─────────────────────────────────────────

        for text in [
            "AES-256 symmetric encryption secures data at rest with 256-bit keys. CBC mode \
             chains blocks but requires initialization vectors. GCM mode provides both \
             encryption and authentication in one operation.",
            "RSA public key cryptography uses paired keys for encryption and signing. Key \
             sizes of 2048 bits minimum are recommended. Elliptic curve cryptography provides \
             equivalent security with shorter keys.",
            "Cross-site scripting prevention requires escaping output in HTML contexts. Content \
             Security Policy headers restrict script sources. Sanitize user input on both \
             client and server sides.",
            "CSRF tokens embedded in forms prevent cross-site request forgery by verifying \
             request origin. SameSite cookie attributes provide additional protection. \
             Double-submit cookie patterns work for stateless APIs.",
            "Parameterized SQL queries prevent injection by separating code from data. ORM \
             frameworks generate parameterized queries automatically. Never concatenate user \
             input into SQL strings directly.",
            "Role-based access control assigns permissions to roles rather than individual \
             users. Users inherit permissions through role membership. Principle of least \
             privilege limits access to what's necessary.",
            "CORS headers control which origins can make cross-origin requests to your API. \
             Preflight OPTIONS requests check allowed methods and headers. Credentials \
             require explicit Access-Control-Allow-Credentials.",
            "Content Security Policy headers specify allowed sources for scripts, styles, and \
             media. Report-only mode logs violations without blocking. Nonce-based policies \
             allow specific inline scripts.",
            "Password hashing with Argon2 or scrypt resists brute-force attacks through memory \
             hardness. Salt each password uniquely before hashing. Never store plaintext \
             passwords or use fast hash functions like MD5.",
            "Time-based one-time passwords generate codes from a shared secret and current \
             time. TOTP apps like Authy and Google Authenticator implement RFC 6238. \
             Recovery codes provide backup access if devices are lost.",
            "Penetration testing simulates real-world attacks to identify security weaknesses. \
             Scope agreements define which systems are in bounds. Findings are classified \
             by severity: critical, high, medium, low.",
            "OWASP Top 10 lists the most critical web application security risks including \
             injection, broken authentication, and sensitive data exposure. Regular security \
             audits check for these common vulnerabilities.",
            "Cryptographic key rotation limits exposure if keys are compromised. Automated \
             rotation schedules replace keys without service disruption. Key versioning \
             allows decryption of data encrypted with previous keys.",
            "X.509 certificate management includes issuance, renewal, and revocation. OCSP \
             and CRL check certificate validity status. Automated renewal with certbot \
             prevents expiration outages.",
            "Security response headers like X-Frame-Options, X-Content-Type-Options, and \
             Strict-Transport-Security harden web applications against common attacks. \
             Referrer-Policy controls information leakage.",
            "Input validation rejects malformed data at application boundaries. Allowlist \
             validation is safer than blocklist approaches. Schema validation libraries \
             enforce structure on JSON payloads.",
            "Audit logging records who did what, when, and from where for compliance and \
             forensics. Tamper-evident logs use append-only storage. Log shipping to a \
             separate system prevents attacker manipulation.",
            "HashiCorp Vault manages secrets, encryption keys, and database credentials with \
             lease-based access. Dynamic secrets are generated on demand and automatically \
             revoked. Transit engine encrypts data without exposing keys.",
            "Zero trust architecture verifies every request regardless of network location. \
             Microsegmentation isolates workloads. Continuous authentication replaces \
             perimeter-based trust models.",
            "Software supply chain security verifies the integrity of dependencies and build \
             artifacts. SBOM lists all components in a software product. Sigstore signs \
             and verifies artifacts cryptographically.",
        ] {
            push_msg(&mut docs, text, "/corpus/bulk/sec");
        }

        // ── Cloud & infrastructure (20 docs) ───────────────────────────

        for text in [
            "AWS S3 provides object storage with eleven nines of durability. Bucket policies \
             control access at the resource level. Lifecycle rules automatically transition \
             objects to cheaper storage tiers.",
            "AWS Lambda runs code in response to events without provisioning servers. Cold \
             starts add latency on first invocation. Provisioned concurrency keeps functions \
             warm for consistent performance.",
            "EC2 instances run virtual servers with configurable CPU, memory, and storage. \
             Spot instances offer unused capacity at steep discounts. Auto-scaling groups \
             adjust instance count based on demand metrics.",
            "Google Cloud Run deploys containerized applications with automatic scaling to \
             zero. Requests trigger cold starts when no instances are running. Concurrency \
             settings control how many requests each instance handles.",
            "Azure Functions supports multiple languages and triggers including HTTP, timer, \
             and queue messages. Durable Functions enable stateful orchestration workflows. \
             Consumption plan charges only for execution time.",
            "Content delivery networks cache responses at edge locations worldwide. Cache-Control \
             headers specify freshness policies. Cache purging invalidates stale content \
             across all edge nodes.",
            "Serverless cold start latency depends on runtime, package size, and cloud provider. \
             Keep deployment packages small and use lighter runtimes. Connection pooling \
             across invocations reduces database overhead.",
            "Auto-scaling policies define rules for adding or removing compute resources based \
             on CPU utilization, queue depth, or custom metrics. Cooldown periods prevent \
             thrashing between scale-up and scale-down events.",
            "CloudFormation templates declare AWS infrastructure as JSON or YAML. Stack updates \
             compute change sets before applying modifications. Drift detection identifies \
             manual changes to managed resources.",
            "Container registries like Docker Hub, ECR, and GCR store and distribute container \
             images. Image scanning detects known vulnerabilities. Content trust verifies \
             image authenticity with digital signatures.",
            "Istio service mesh adds observability, traffic management, and security between \
             microservices. Sidecar proxies intercept all network traffic transparently. \
             Circuit breakers prevent cascading failures across services.",
            "API gateways handle authentication, rate limiting, and request routing for \
             backend services. Kong, AWS API Gateway, and Envoy are popular choices. \
             Request transformation adapts payloads between API versions.",
            "Cloud cost optimization identifies unused resources and right-sizes instances. \
             Reserved instances and savings plans reduce costs for predictable workloads. \
             Tagging resources enables cost allocation by team or project.",
            "Multi-region deployments distribute applications across geographic zones for \
             disaster recovery and lower latency. Data replication strategies balance \
             consistency with availability. Active-active configurations serve traffic \
             from all regions.",
            "Disaster recovery plans define RPO and RTO targets for business continuity. \
             Backup strategies include full, incremental, and differential approaches. \
             Regular DR drills validate recovery procedures work correctly.",
            "Managed database services like RDS and Cloud SQL handle backups, patching, and \
             replication automatically. Read replicas distribute query load. Multi-AZ \
             deployments provide automatic failover.",
            "Object storage lifecycle policies automatically archive infrequently accessed \
             data. Glacier and Archive tiers offer low-cost long-term storage. Retrieval \
             times vary from minutes to hours based on tier.",
            "Virtual private clouds isolate network resources with subnets, route tables, and \
             security groups. VPC peering connects clouds across accounts or regions. \
             Private endpoints keep traffic off the public internet.",
            "IAM policies use JSON documents to define who can access which resources and \
             actions. Roles delegate permissions without sharing credentials. Multi-factor \
             authentication adds a second verification step.",
            "Cloud monitoring aggregates metrics, logs, and traces from distributed services. \
             Dashboards visualize system health in real time. Anomaly detection alerts on \
             unexpected metric patterns before users notice problems.",
        ] {
            push_msg(&mut docs, text, "/corpus/bulk/cloud");
        }

        // ── Data structures & algorithms (20 docs) ─────────────────────

        for text in [
            "Hash maps store key-value pairs with average O(1) lookup time. Hash collisions \
             use chaining or open addressing strategies. Load factor determines when to \
             resize the underlying array.",
            "Binary search trees maintain sorted order with O(log n) average operations. \
             Unbalanced trees degrade to O(n) linked list performance. Self-balancing \
             variants like AVL and red-black trees prevent this.",
            "Graph traversal with BFS explores nodes level by level using a queue. DFS follows \
             paths to maximum depth using a stack or recursion. Both visit every reachable \
             node exactly once.",
            "Linked lists provide O(1) insertion and deletion at known positions. No random \
             access means traversal is required to find elements. Doubly-linked lists \
             support backward traversal at the cost of extra pointers.",
            "Min-heaps support O(1) minimum lookup and O(log n) insertion and extraction. \
             Priority queues are commonly implemented with binary heaps. Heapify builds a \
             heap from an unsorted array in O(n) time.",
            "Comparison-based sorting algorithms cannot exceed O(n log n) average performance. \
             Quicksort uses partitioning around pivots. Mergesort guarantees O(n log n) \
             worst case through recursive division and merging.",
            "Dynamic programming solves problems by breaking them into overlapping subproblems. \
             Memoization caches results of expensive function calls. Bottom-up tabulation \
             fills a table iteratively without recursion overhead.",
            "Trie data structures store strings character by character in a tree. Prefix \
             matching finds all strings starting with a given prefix efficiently. Compressed \
             tries reduce memory by merging single-child chains.",
            "Red-black trees maintain balance through node coloring invariants. Rotations \
             and recoloring restore balance after insertions and deletions. Guaranteed \
             O(log n) height ensures consistent operation times.",
            "Bloom filters test set membership with no false negatives but possible false \
             positives. Multiple hash functions map elements to bit positions. Space \
             efficiency makes them useful for cache lookup and deduplication.",
            "Skip lists use layered linked lists for O(log n) probabilistic search performance. \
             Higher lanes skip over multiple elements for faster traversal. Insertion \
             randomly promotes elements to higher levels.",
            "Union-find tracks disjoint sets with near O(1) amortized operations. Path \
             compression flattens tree structures during find operations. Union by rank \
             keeps trees shallow when merging sets.",
            "Topological sorting orders directed acyclic graph nodes so dependencies come \
             first. Kahn's algorithm uses in-degree tracking. Cycle detection identifies \
             circular dependencies that prevent valid ordering.",
            "A* pathfinding combines actual distance with heuristic estimates to find optimal \
             paths. Admissible heuristics guarantee shortest path discovery. The algorithm \
             expands the most promising nodes first using a priority queue.",
            "LRU cache evicts the least recently used entry when capacity is reached. \
             Combining a hash map with a doubly-linked list achieves O(1) get and put. \
             Cache hit rate measures the effectiveness of the eviction policy.",
            "Segment trees answer range queries like sum, minimum, and maximum in O(log n) \
             time. Lazy propagation defers updates until queried. Persistent segment trees \
             retain previous versions for time-travel queries.",
            "Consistent hashing distributes data across nodes with minimal redistribution when \
             nodes join or leave. Virtual nodes improve balance across physical servers. \
             Used by distributed caches and databases.",
            "B-trees store sorted data with branching factor optimized for disk block sizes. \
             Internal nodes hold keys and child pointers. B+ trees store all values in \
             leaves with linked-list traversal for range scans.",
            "Amortized analysis averages operation costs over a sequence to show that expensive \
             operations happen infrequently. Dynamic array resizing is O(1) amortized despite \
             occasional O(n) copy operations.",
            "Space-time tradeoffs let algorithms use more memory for faster execution or less \
             memory at the cost of speed. Lookup tables precompute results. Streaming \
             algorithms process data in a single pass with bounded memory.",
        ] {
            push_msg(&mut docs, text, "/corpus/bulk/algo");
        }

        // ── Operating systems (15 docs) ────────────────────────────────

        for text in [
            "Process schedulers allocate CPU time using algorithms like round-robin, priority \
             queues, and completely fair scheduling. Context switches save and restore register \
             state between processes.",
            "Virtual memory maps process address spaces to physical RAM through page tables. \
             Page faults trigger the OS to load pages from disk. Translation lookaside \
             buffers cache recent virtual-to-physical mappings.",
            "File system inodes store metadata including permissions, timestamps, and block \
             pointers. Directories map filenames to inode numbers. Hard links share inodes \
             while soft links point to path names.",
            "System calls provide the interface between user-space applications and the kernel. \
             Read, write, open, and close are fundamental file operations. Syscall overhead \
             is minimized through batching and io_uring.",
            "Inter-process communication mechanisms include pipes, message queues, shared memory, \
             and Unix domain sockets. Named pipes persist in the filesystem. Signal handlers \
             respond to asynchronous notifications.",
            "POSIX signal handling registers callbacks for events like SIGTERM, SIGINT, and \
             SIGHUP. Signal masks block delivery during critical sections. Real-time signals \
             queue rather than merge pending deliveries.",
            "Berkeley sockets provide the API for TCP and UDP network programming. Bind, \
             listen, and accept establish server-side connections. Non-blocking sockets \
             and select/poll enable concurrent connection handling.",
            "Mutex locks enforce mutual exclusion for shared resource access between threads. \
             Condition variables allow threads to wait for specific state changes. Read-write \
             locks permit concurrent reads with exclusive writes.",
            "Memory-mapped files let applications access file contents through virtual memory \
             addresses. The OS pages data in and out transparently. mmap avoids explicit \
             read and write system calls for large files.",
            "Loadable kernel modules extend OS functionality without rebooting. Device drivers, \
             filesystems, and network protocols can be loaded dynamically. Module dependencies \
             are resolved automatically by the module loader.",
            "Linux namespaces isolate process groups for containers. PID namespaces give each \
             container its own process tree. Network namespaces provide separate network \
             stacks with independent routing tables.",
            "Control groups limit and account for CPU, memory, and I/O resources consumed \
             by process groups. Hierarchical cgroups inherit parent limits. Container \
             runtimes use cgroups to enforce resource constraints.",
            "Event-driven I/O with epoll and kqueue efficiently monitors thousands of file \
             descriptors. Edge-triggered mode notifies only on state changes. io_uring \
             provides zero-copy async I/O with ring buffers.",
            "Journaling file systems like ext4 and XFS log metadata changes before committing \
             them. Write-ahead logging prevents corruption during unexpected power loss. \
             Copy-on-write filesystems like ZFS never overwrite existing data.",
            "Swap space extends available memory by moving inactive pages to disk. Swappiness \
             controls how aggressively the kernel swaps pages. SSDs reduce swap latency \
             compared to traditional hard drives.",
        ] {
            push_msg(&mut docs, text, "/corpus/bulk/os");
        }

        // ── DevOps & monitoring tools (15 docs) ────────────────────────

        for text in [
            "Ansible playbooks define infrastructure automation as YAML declarative tasks. \
             Agentless design connects to hosts via SSH. Roles organize reusable collections \
             of tasks, handlers, and templates.",
            "Chef cookbooks describe system configuration with Ruby-based recipes. Knife CLI \
             manages nodes, roles, and data bags. Test Kitchen validates cookbooks in \
             isolated virtual environments.",
            "Puppet manifests declare the desired state of system resources in a domain-specific \
             language. Agents pull configurations from the Puppet master periodically. Facter \
             collects system facts for conditional configuration.",
            "Vagrant creates reproducible development environments using VirtualBox, VMware, \
             or Docker providers. Vagrantfiles define machine configuration. Synced folders \
             share code between host and guest machines.",
            "Packer builds identical machine images for multiple platforms from a single source \
             configuration. Builders target AWS AMIs, Docker images, and VMware templates. \
             Provisioners run shell scripts or Ansible playbooks during image creation.",
            "Consul provides service discovery, health checking, and distributed key-value \
             storage. DNS and HTTP interfaces locate healthy service instances. Consul \
             Connect adds mutual TLS between services automatically.",
            "Prometheus recording rules precompute frequently needed expressions for faster \
             dashboard loading. Alertmanager groups and routes alerts to notification \
             channels. Silence rules temporarily suppress known alerts during maintenance.",
            "Elasticsearch indexes JSON documents for full-text search and analytics. Shards \
             distribute data across nodes for horizontal scaling. Mappings define field \
             types and analysis settings for each index.",
            "Logstash pipelines parse, transform, and route log data between inputs and \
             outputs. Grok patterns extract structured fields from unstructured text. \
             Conditional logic routes events to different outputs.",
            "Kibana dashboards visualize Elasticsearch data with charts, tables, and maps. \
             Saved searches share common query patterns across teams. Lens provides \
             drag-and-drop visualization building.",
            "Datadog unified monitoring combines metrics, logs, and traces in a single \
             platform. Custom metrics track business-specific KPIs. APM distributed \
             tracing follows requests across service boundaries.",
            "New Relic application performance monitoring instruments code to measure response \
             times and throughput. Error analytics group and prioritize runtime exceptions. \
             Browser monitoring tracks frontend performance from real users.",
            "PagerDuty incident management routes alerts to on-call engineers with escalation \
             policies. Runbooks document troubleshooting steps for common incidents. \
             Postmortems capture lessons learned after resolution.",
            "Statuspage communicates service health to users during incidents and maintenance \
             windows. Component-level status shows which features are affected. Automated \
             monitoring integrations update status based on alerts.",
            "Jaeger distributed tracing visualizes request flows across microservices. Span \
             context propagates through HTTP headers and message queues. Sampling strategies \
             balance observability coverage with storage costs.",
        ] {
            push_msg(&mut docs, text, "/corpus/bulk/devops");
        }

        // ── Mobile development (10 docs) ───────────────────────────────

        for text in [
            "iOS development with UIKit uses view controllers to manage screen content. \
             Auto Layout constraints define responsive layouts. Storyboards provide visual \
             interface building with segue transitions.",
            "Android Activities represent single screens with lifecycle callbacks for creation, \
             pausing, and destruction. Fragments enable reusable UI components. Jetpack \
             libraries standardize common patterns.",
            "React Native bridges JavaScript UI components to native platform controls. Hot \
             reloading applies code changes without rebuilding. Native modules expose \
             platform-specific functionality to JavaScript.",
            "Flutter's rendering engine draws every pixel directly, bypassing platform UI \
             frameworks. StatefulWidget and StatelessWidget manage component state. \
             Platform channels call native code when needed.",
            "Mobile app deep linking opens specific content from URLs or other apps. Universal \
             links on iOS and app links on Android verify domain ownership. Deferred deep \
             links work even when the app isn't installed yet.",
            "Push notification services deliver messages through APNs on iOS and FCM on \
             Android. Notification channels on Android categorize alerts. Background \
             data notifications trigger silent content updates.",
            "Mobile app offline storage uses SQLite, Realm, or Core Data for structured data. \
             Sync strategies reconcile local changes with server state. Conflict resolution \
             policies handle concurrent modifications.",
            "App store submission requires code signing, provisioning profiles, and metadata. \
             TestFlight distributes iOS beta builds to testers. Google Play internal testing \
             tracks provide staged rollouts.",
            "Mobile performance profiling with Instruments on iOS and Android Profiler measures \
             CPU, memory, and battery usage. Reduce main thread blocking for smooth 60fps \
             animations. Image caching avoids repeated network downloads.",
            "Responsive mobile layouts adapt to different screen sizes and orientations. Safe \
             area insets account for notches and system bars. Dynamic type scales fonts based \
             on user accessibility preferences.",
        ] {
            push_msg(&mut docs, text, "/corpus/bulk/mobile");
        }

        // ── Web frameworks & patterns (15 docs) ────────────────────────

        for text in [
            "Next.js server-side rendering generates HTML on each request for SEO and fast \
             initial loads. Static site generation pre-renders pages at build time. \
             Incremental static regeneration updates pages without full rebuilds.",
            "Django ORM maps Python classes to database tables with migration support. \
             QuerySet chaining builds complex queries lazily. The admin interface \
             auto-generates CRUD views from models.",
            "Express.js middleware functions process HTTP requests in a pipeline. Routers \
             organize endpoints into modular groups. Error-handling middleware catches \
             exceptions from route handlers.",
            "Vue.js reactive data binding automatically updates the DOM when state changes. \
             Single-file components combine template, script, and style. Vuex provides \
             centralized state management.",
            "Svelte compiles components to efficient JavaScript at build time without a \
             virtual DOM runtime. Reactive declarations with $: automatically track \
             dependencies. SvelteKit adds routing and server-side rendering.",
            "FastAPI generates OpenAPI documentation automatically from Python type hints. \
             Pydantic models validate request and response data. Async route handlers \
             support concurrent request processing.",
            "Ruby on Rails convention over configuration reduces boilerplate decisions. Active \
             Record pattern maps objects to database rows. Generators scaffold models, \
             controllers, and views from the command line.",
            "Angular dependency injection provides services to components through constructor \
             parameters. RxJS observables handle asynchronous data streams. Ahead-of-time \
             compilation optimizes bundle size.",
            "Remix loaders fetch data on the server before rendering routes. Action functions \
             handle form submissions. Nested routes compose layouts for shared UI sections.",
            "htmx extends HTML with AJAX attributes for dynamic interactions without writing \
             JavaScript. hx-get, hx-post, and hx-swap control request behavior and DOM \
             updates declaratively.",
            "Tailwind CSS utility classes compose styles directly in HTML markup. JIT mode \
             generates only used styles for minimal bundle size. Configuration files \
             customize the design system tokens.",
            "Astro delivers zero JavaScript by default for content-focused websites. Island \
             architecture hydrates only interactive components. Multiple framework \
             components coexist on the same page.",
            "Session management stores user state between HTTP requests using server-side \
             stores or signed cookies. Sliding expiration extends sessions on activity. \
             Session fixation prevention regenerates IDs after authentication.",
            "Cross-origin resource sharing preflight requests check whether the actual request \
             is permitted. Wildcard origins work for public APIs but not with credentials. \
             Vary: Origin headers prevent cache poisoning.",
            "Web accessibility ensures content is usable by people with disabilities. ARIA \
             attributes convey semantic meaning to assistive technologies. Keyboard navigation \
             and sufficient color contrast are fundamental requirements.",
        ] {
            push_msg(&mut docs, text, "/corpus/bulk/web");
        }

        // ── More database topics (10 docs) ─────────────────────────────

        for text in [
            "PostgreSQL JSONB columns store and query semi-structured data with GIN indexes. \
             CTEs organize complex queries into readable named subqueries. Window functions \
             compute running totals and rankings.",
            "MySQL InnoDB engine provides row-level locking and MVCC for concurrent \
             transactions. The query optimizer uses cost-based analysis to choose execution \
             plans. EXPLAIN shows the selected access paths.",
            "MongoDB document model stores data as flexible BSON objects in collections. \
             Aggregation pipelines transform and analyze documents through stages. Replica \
             sets provide automatic failover for high availability.",
            "DynamoDB provides single-digit millisecond key-value and document operations at \
             any scale. Partition keys distribute data across storage nodes. Global secondary \
             indexes enable queries on non-key attributes.",
            "CockroachDB distributes SQL across multiple nodes with serializable transactions. \
             Automatic range splitting balances data as tables grow. Geo-partitioning pins \
             data to specific regions for compliance.",
            "TimescaleDB extends PostgreSQL with time-series optimizations. Hypertables \
             automatically partition data by time intervals. Continuous aggregates \
             incrementally maintain materialized views.",
            "Neo4j stores data as nodes and relationships in a property graph. Cypher query \
             language uses ASCII-art patterns for intuitive graph traversal. Index-free \
             adjacency provides constant-time relationship lookups.",
            "ClickHouse columnar storage achieves high compression ratios for analytical \
             queries. Materialized views precompute aggregations on insert. Distributed \
             tables span queries across multiple shards.",
            "SQLite WAL mode allows concurrent readers and a single writer without blocking. \
             The journal file tracks uncommitted changes for crash recovery. VACUUM \
             reclaims unused space and defragments the database file.",
            "Database sharding partitions data horizontally across multiple servers. Shard \
             keys determine data placement. Cross-shard queries require scatter-gather \
             coordination and are significantly more expensive.",
        ] {
            push_msg(&mut docs, text, "/corpus/bulk/db");
        }

        // ── Testing & QA (10 docs) ─────────────────────────────────────

        for text in [
            "Unit tests isolate individual functions by mocking dependencies. Test doubles \
             include stubs, mocks, fakes, and spies. Dependency injection makes components \
             testable by decoupling creation from use.",
            "Integration tests verify that components work together correctly across module \
             boundaries. Test databases provide isolated environments. Fixtures set up known \
             state before each test run.",
            "End-to-end tests simulate real user workflows through the complete application \
             stack. Playwright and Cypress automate browser interactions. Flaky tests from \
             timing issues undermine suite reliability.",
            "Test coverage measures the percentage of code executed during tests. Line, branch, \
             and condition coverage provide different perspectives. High coverage doesn't \
             guarantee correctness but low coverage indicates gaps.",
            "Mutation testing injects small code changes to verify that tests detect them. \
             Surviving mutants indicate weak test assertions. Mutation scores complement \
             traditional coverage metrics.",
            "Snapshot testing captures component output and compares against saved references. \
             Visual regression testing detects unintended UI changes. Updating snapshots \
             accepts intentional changes as the new baseline.",
            "Load testing simulates concurrent users to measure system performance under \
             stress. Tools like k6, Gatling, and Locust generate realistic traffic patterns. \
             Percentile latency metrics reveal tail performance.",
            "Fuzzing generates random or semi-structured inputs to discover crashes and edge \
             cases. Coverage-guided fuzzers prioritize inputs that explore new code paths. \
             Sanitizers detect memory errors during fuzz runs.",
            "Contract testing verifies API compatibility between consumer and provider without \
             integration. Pact generates consumer-driven contracts that providers verify \
             independently. Schema evolution avoids breaking changes.",
            "Accessibility testing verifies screen reader compatibility and keyboard navigation. \
             Automated tools like axe-core catch WCAG violations. Manual testing by users \
             with disabilities provides qualitative insights.",
        ] {
            push_msg(&mut docs, text, "/corpus/bulk/test");
        }

        // ── More error messages (20 docs) ──────────────────────────────

        for text in [
            "ConnectionRefusedError: No connection could be made because the target machine \
             actively refused it at 127.0.0.1:5432. Check that the database server is running.",
            "TimeoutError: Operation timed out after 30000ms waiting for response from \
             api.example.com. The server may be overloaded or unreachable.",
            "PermissionError: [Errno 13] Permission denied: '/etc/nginx/nginx.conf'. Run with \
             sudo or check file ownership and permissions.",
            "OutOfMemoryError: Java heap space. The JVM could not allocate an object in the \
             heap. Increase -Xmx or investigate memory leaks with a heap dump.",
            "FileNotFoundError: [Errno 2] No such file or directory: 'requirements.txt'. \
             Verify the working directory and file path.",
            "json.decoder.JSONDecodeError: Expecting value at line 1 column 1. The response \
             body is empty or contains invalid JSON.",
            "ModuleNotFoundError: No module named 'pandas'. Install with pip install pandas \
             or check that the virtual environment is activated.",
            "RuntimeError: CUDA out of memory. Tried to allocate 2.00 GiB. Reduce batch size \
             or use gradient checkpointing to lower GPU memory usage.",
            "AssertionError: Expected status code 200 but got 403. The authentication token \
             may have expired or the user lacks required permissions.",
            "IndexError: list index out of range at line 87. The list has fewer elements than \
             expected. Add bounds checking before accessing by index.",
            "NullPointerException at com.app.service.UserService.getProfile(UserService.java:42). \
             The user object was null. Add null checks or use Optional.",
            "error[E0382]: borrow of moved value: `data`. The value was moved in the previous \
             line. Clone the value or use references to avoid ownership transfer.",
            "panic: runtime error: index out of range [5] with length 3. Go slice access \
             exceeded bounds. Check length before indexing.",
            "FATAL: password authentication failed for user 'postgres'. Verify pg_hba.conf \
             settings and the database user credentials.",
            "Error: ENOSPC: no space left on device, write. The disk is full. Free space by \
             removing logs, temp files, or old deployments.",
            "AuthenticationError: Invalid API key provided. Check that the key is correct and \
             has not been revoked. Rotate keys if exposed.",
            "429 Too Many Requests: Rate limit exceeded. Retry after 60 seconds. Implement \
             exponential backoff with jitter for retry logic.",
            "ssl.SSLCertVerificationError: certificate verify failed. The SSL certificate \
             has expired or the certificate chain is incomplete.",
            "ERROR 1213 (40001): Deadlock found when trying to get lock. InnoDB detected a \
             circular wait. Retry the transaction or reorder operations.",
            "ConcurrentModificationException: HashMap was modified during iteration. Use \
             ConcurrentHashMap or collect modifications and apply after iteration.",
        ] {
            let err_idx = docs.len();
            docs.push(
                Document::new(
                    ChunkKind::ToolResult,
                    text.to_string(),
                    PathBuf::from(format!("/corpus/bulk/errors/{err_idx}.jsonl")),
                )
                .with_is_error(Some(true))
                .with_tool_name(Some("Bash".to_string())),
            );
        }

        // ── More tool invocations (20 docs) ────────────────────────────

        for (tool, content, input) in [
            (
                "Bash",
                "npm install express",
                r#"{"command":"npm install express"}"#,
            ),
            (
                "Bash",
                "docker build -t myapp .",
                r#"{"command":"docker build -t myapp ."}"#,
            ),
            (
                "Bash",
                "kubectl get pods -n production",
                r#"{"command":"kubectl get pods -n production"}"#,
            ),
            (
                "Bash",
                "python -m pytest tests/ -v",
                r#"{"command":"python -m pytest tests/ -v"}"#,
            ),
            (
                "Bash",
                "curl -s https://api.example.com/health",
                r#"{"command":"curl -s https://api.example.com/health"}"#,
            ),
            (
                "Bash",
                "psql -c 'SELECT count(*) FROM users'",
                r#"{"command":"psql -c 'SELECT count(*) FROM users'"}"#,
            ),
            (
                "Bash",
                "terraform plan -out=tfplan",
                r#"{"command":"terraform plan -out=tfplan"}"#,
            ),
            (
                "Bash",
                "ansible-playbook deploy.yml",
                r#"{"command":"ansible-playbook deploy.yml"}"#,
            ),
            (
                "Bash",
                "go test ./... -race -cover",
                r#"{"command":"go test ./... -race -cover"}"#,
            ),
            (
                "Bash",
                "openssl s_client -connect example.com:443",
                r#"{"command":"openssl s_client -connect example.com:443"}"#,
            ),
            (
                "Read",
                "/app/src/routes/api.ts",
                r#"{"file_path":"/app/src/routes/api.ts"}"#,
            ),
            (
                "Read",
                "/etc/nginx/nginx.conf",
                r#"{"file_path":"/etc/nginx/nginx.conf"}"#,
            ),
            ("Read", "Dockerfile", r#"{"file_path":"Dockerfile"}"#),
            (
                "Read",
                "/app/package.json",
                r#"{"file_path":"/app/package.json"}"#,
            ),
            (
                "Grep",
                "pattern: TODO|FIXME|HACK",
                r#"{"pattern":"TODO|FIXME|HACK","path":"src/"}"#,
            ),
            (
                "Grep",
                "pattern: import.*React",
                r#"{"pattern":"import.*React","path":"src/components"}"#,
            ),
            (
                "Edit",
                "adding error boundary wrapper",
                r#"{"file_path":"src/App.tsx","old_string":"<Router>","new_string":"<ErrorBoundary><Router>"}"#,
            ),
            (
                "Edit",
                "fixing database connection string",
                r#"{"file_path":".env","old_string":"DB_HOST=localhost","new_string":"DB_HOST=db.internal"}"#,
            ),
            (
                "Write",
                "creating migration file",
                r#"{"file_path":"migrations/003_add_index.sql"}"#,
            ),
            (
                "Write",
                "creating test fixture",
                r#"{"file_path":"tests/fixtures/users.json"}"#,
            ),
        ] {
            let tool_idx = docs.len();
            docs.push(
                Document::new(
                    ChunkKind::ToolUse,
                    content.to_string(),
                    PathBuf::from(format!("/corpus/bulk/tools/{tool_idx}.jsonl")),
                )
                .with_tool_name(Some(tool.to_string()))
                .with_tool_id(Some(format!("tool-{tool_idx:04}")))
                .with_tool_input(Some(input.to_string())),
            );
        }

        // ── Cross-cutting / multi-topic docs (20 docs) ─────────────────

        for text in [
            "Using Docker to containerize a Rust web server built with Actix-Web. The multi-stage \
             Dockerfile compiles with cargo build --release then copies the binary into a minimal \
             Alpine image.",
            "Git pre-commit hooks run cargo fmt and cargo clippy before allowing commits. This \
             prevents poorly formatted or warning-laden code from entering the repository.",
            "Deploying a Python FastAPI application to AWS Lambda using Mangum adapter. API Gateway \
             routes HTTP requests to the Lambda function with cold start optimization.",
            "Setting up PostgreSQL full-text search as an alternative to Elasticsearch for \
             smaller deployments. tsvector columns and GIN indexes power the search.",
            "Migrating a monolithic Rails application to microservices with gRPC communication. \
             Shared protobuf definitions ensure type-safe contracts between services.",
            "Implementing OAuth2 login with GitHub as the identity provider for a Next.js \
             application. NextAuth.js handles the callback flow and session management.",
            "Kubernetes deployment manifests for a Redis cluster with persistent volume claims. \
             StatefulSets maintain stable network identities across pod restarts.",
            "Performance profiling a Node.js API with clinic.js to identify event loop blocking. \
             Moving CPU-intensive operations to worker threads improved p99 latency.",
            "Writing Terraform modules for provisioning VPC, subnets, and security groups on \
             AWS. Module inputs parameterize CIDR blocks and availability zones.",
            "Configuring Prometheus to scrape metrics from a Go application using the official \
             client library. Custom histograms track request duration by endpoint.",
            "CI/CD pipeline with GitHub Actions that runs Rust tests, builds a Docker image, \
             and deploys to Kubernetes on merge to main.",
            "Setting up Grafana alerts on database connection pool exhaustion detected through \
             application metrics exposed via the /metrics endpoint.",
            "Implementing rate limiting with Redis sorted sets for sliding window counters. \
             Each API key gets independent limits configured per endpoint.",
            "Testing a React component that makes API calls using Mock Service Worker to \
             intercept network requests with realistic response fixtures.",
            "Using SQLite with WAL mode for an embedded analytics engine in a desktop \
             application. FTS5 enables full-text search across imported documents.",
            "Debugging a memory leak in a long-running Python process using objgraph and \
             tracemalloc. Circular references in cached objects prevented garbage collection.",
            "Configuring nginx reverse proxy with SSL termination, gzip compression, and \
             WebSocket upgrade support for a real-time chat application.",
            "Building a CLI tool in Go with cobra that interacts with a REST API. Viper \
             handles configuration from environment variables and config files.",
            "Implementing event sourcing with Apache Kafka as the event store. Consumers \
             rebuild aggregate state by replaying events from topic partitions.",
            "Setting up Dependabot for automated dependency updates with auto-merge for \
             patch versions. Security advisories trigger immediate pull requests.",
        ] {
            push_msg(&mut docs, text, "/corpus/bulk/cross");
        }

        // ── Conversational / meta docs (15 docs) ───────────────────────

        for text in [
            "Let me think about the best approach for this refactoring. We could extract a \
             trait or use an enum, but the trait approach seems more extensible.",
            "I looked into the failing test and the issue is a race condition in the setup \
             code. The database connection isn't ready when the assertions run.",
            "Good question about the architecture. The current design separates the ingestion \
             pipeline from the query path so they can scale independently.",
            "I would recommend starting with the simplest solution and only adding complexity \
             when the profiler shows it's actually a bottleneck.",
            "The error you're seeing is because the environment variable isn't set in the CI \
             environment. Add it to the GitHub Actions secrets.",
            "Looking at the git history, this function was refactored three times last month. \
             The latest version removed the caching layer that was causing stale data bugs.",
            "I can reproduce the bug locally. It only happens when the request body exceeds \
             the default buffer size of 8KB. Increasing the limit fixes it.",
            "After reviewing the pull request, I think we should split it into two commits: \
             one for the database migration and one for the application code changes.",
            "The documentation says to use version 3 of the API but the SDK still defaults \
             to version 2. You need to pass the version explicitly in the config.",
            "This is a known limitation of the library. The maintainer opened an issue about \
             it six months ago but there's no fix yet. We could fork or find an alternative.",
            "The performance regression was introduced in commit abc123 when we switched from \
             a hash map to a sorted vector. Reverting that change restored the original latency.",
            "I tested both approaches and the async version is actually slower for our use case \
             because the overhead of task spawning exceeds the I/O wait time.",
            "Based on the flame graph, 60% of CPU time is spent in JSON serialization. \
             Switching to simd-json or using a binary format would help significantly.",
            "The dependency graph shows a circular reference between the auth and user modules. \
             Extracting a shared interface module would break the cycle cleanly.",
            "We should add retry logic with exponential backoff for the external API calls. \
             The current code fails immediately on any transient network error.",
        ] {
            push_msg(&mut docs, text, "/corpus/bulk/conv");
        }

        // ── Expanded workplace / noise filler (100 docs) ───────────────

        for text in [
            "Grabbed tacos from the food truck for lunch. Their habanero salsa is incredible.",
            "The elevator is out of service again. Taking the stairs to the fourth floor.",
            "Finally finished organizing the storage closet. Found three old keyboards.",
            "The sunrise over the mountains this morning was absolutely stunning.",
            "Picked up the dry cleaning on the way to the office before the store closed.",
            "The dog needs to go to the vet for annual vaccinations next Wednesday.",
            "Trying a new recipe for sourdough bread this weekend using a wild yeast starter.",
            "The neighbors are renovating their kitchen and the noise starts at seven AM.",
            "Signed up for the company volleyball league. Games are Thursday evenings.",
            "The flight to Seattle got delayed by two hours due to fog at the airport.",
            "Reorganized my bookshelf by genre instead of alphabetical order.",
            "The farmer's market on Saturday had amazing heirloom tomatoes and fresh basil.",
            "Need to schedule an oil change for the car before the road trip next month.",
            "The sunset from the rooftop terrace was beautiful shades of orange and purple.",
            "Dropped off the package at the post office during my lunch break.",
            "The hiking trail to the waterfall is about four miles round trip.",
            "Tried the new Thai restaurant downtown. The pad see ew was excellent.",
            "The gym is offering a free trial week for their new spin classes.",
            "Watered the tomato plants on the balcony. They're starting to produce fruit.",
            "The city council approved the new bike lane on the main boulevard.",
            "Finished the mystery novel I've been reading. The ending was surprising.",
            "The power went out for about thirty minutes during the thunderstorm last night.",
            "Taking the train instead of driving tomorrow because of construction on the highway.",
            "The company picnic at the lake was a nice break from the usual routine.",
            "Refilled my prescription at the pharmacy on the corner near the office.",
            "The autumn leaves are starting to change colors in the park across the street.",
            "Ordered a new standing desk converter that should arrive by Friday.",
            "The live music at the brewery was really good. They play every Tuesday night.",
            "Fixed the leaky faucet in the bathroom. Just needed a new washer ring.",
            "The public library is hosting a book club for science fiction on the first Monday.",
            "Volunteered at the food bank this weekend. They always need help on Saturdays.",
            "The new Thai place does delivery through two different apps now.",
            "Cleaned out the garage and donated a bunch of old clothes and shoes.",
            "The commute was unusually smooth today. Got to the office fifteen minutes early.",
            "Planning a camping trip to the national park for the long weekend.",
            "The office building is switching to LED lighting to reduce energy costs.",
            "Bought tickets for the concert next month at the outdoor amphitheater.",
            "The roof needs inspection after the recent hailstorm. Calling a contractor.",
            "Started learning watercolor painting from YouTube tutorials. It's therapeutic.",
            "The neighborhood watch meeting is scheduled for this Thursday evening at seven.",
            "Ran five kilometers this morning along the river path before work.",
            "The supermarket started carrying that imported cheese I've been looking for.",
            "The landlord finally fixed the broken intercom system in the building lobby.",
            "Taking a photography class at the community college on Saturday mornings.",
            "The cat knocked over the plant again. Need a more stable pot.",
            "Found a great deal on flights to Portugal for the spring vacation.",
            "The community garden plot is ready for planting after tilling last weekend.",
            "Replaced the batteries in the smoke detectors throughout the apartment.",
            "The basketball game tonight should be exciting. Both teams are undefeated.",
            "Made homemade pasta from scratch for the first time. It took longer than expected.",
            "The wi-fi router keeps dropping connections. Might need to replace it soon.",
            "Signed up for a pottery workshop at the art center downtown.",
            "The snow plow came through at three AM and woke up the entire block.",
            "Found a nice spot for the weekend picnic under the big oak tree in the park.",
            "The annual charity 5K run is in two weeks. Registration closes Friday.",
            "Just finished assembling the new bookcase from the furniture store.",
            "The sushi place near the office has a great happy hour deal until six PM.",
            "Took the bike to the shop for new brake pads and a tire replacement.",
            "The movie we watched last night was surprisingly good for a sequel.",
            "Need to return the library books before the fines start accumulating.",
            "The heating system makes a clicking sound. Called the maintenance company.",
            "Harvested the first batch of peppers from the backyard garden.",
            "The local farmers market is switching to Wednesday evenings for the summer.",
            "Just renewed the gym membership for another year with the early bird discount.",
            "The neighborhood block party is being organized for the end of the month.",
            "Cleaned the gutters this weekend before the rainy season starts.",
            "The museum exhibit on ancient Egypt runs through the end of next month.",
            "Found my old college textbooks in a box in the attic while cleaning.",
            "The pizza delivery took an hour and a half. The food was lukewarm.",
            "Started a jigsaw puzzle with a thousand pieces. It's going to take a while.",
            "The tree in the front yard needs trimming before the branches touch the power lines.",
            "Signed up for a volunteer tutoring program at the local elementary school.",
            "The car wash next to the gas station is running a half-price special this week.",
            "Made reservations at the Italian restaurant for Saturday evening at seven thirty.",
            "The public pool opens for the summer season on Memorial Day weekend.",
            "Reorganized the kitchen cabinets to make better use of the vertical space.",
            "The construction on the intersection is causing major traffic backups.",
            "Found a nice vintage lamp at the antique store on Main Street.",
            "The kids soccer practice is moving to the turf field starting next week.",
            "Installed a shelf in the laundry room for the detergent and supplies.",
            "The bird feeder attracted three different species this morning.",
            "Finally painted the bedroom the light blue color we picked out months ago.",
            "The insurance company needs the repair estimate before approving the claim.",
            "Went to the botanical garden and the orchid section was spectacular.",
            "The plumber is coming Tuesday morning to fix the running toilet.",
            "Organized a carpool with two coworkers who live in the same neighborhood.",
            "The spring cleaning checklist has about thirty items left to complete.",
            "Found a good deal on a used road bike at the cycling shop.",
            "The dinner party went well. Everyone loved the homemade tiramisu.",
            "Need to update the emergency contact information at the doctor's office.",
            "The yard sale raised enough money to cover the school field trip costs.",
            "Washed and waxed the car in the driveway on a perfect sunny afternoon.",
            "The new crossword puzzle book arrived in the mail today.",
            "Scheduled the annual chimney inspection before we start using the fireplace.",
            "The family reunion is planned for the lake house in August.",
            "Hung new curtains in the living room. The old ones were sun-bleached.",
            "The coffee shop down the street started roasting their own beans.",
            "Moved the patio furniture out of storage for the warmer weather.",
            "The optometrist appointment is next Monday at ten in the morning.",
            "The kids built an impressive sandcastle at the beach last weekend.",
            "Replaced the door mat and added a small bench near the front entrance.",
            "The aquarium downtown has a new jellyfish exhibit that opens this weekend.",
            "Planted tulip bulbs along the walkway for spring blooms next year.",
            "The dentist appointment went smoothly. No cavities this time.",
            "Sold the old couch on the marketplace app within two hours of posting.",
            "The thunderstorm knocked out the wi-fi for about twenty minutes last night.",
            "Took the scenic route through the countryside on the drive home.",
            "The hardware store had the exact shade of paint for the bathroom touch-up.",
            "Joined the morning yoga class at the community center on Tuesdays.",
            "The hiking group is planning a trip to the canyon for the holiday weekend.",
            "Picked apples at the orchard and made a homemade pie from scratch.",
            "The parking garage raised rates again. Might switch to the lot two blocks away.",
            "Finished the crossword puzzle in record time this morning over coffee.",
            "The recycling center started accepting electronics on the first Saturday of each month.",
            "Tried rock climbing at the indoor gym. My forearms are still sore.",
            "The street fair this weekend had over fifty different food vendors.",
            "Adopted a rescue dog from the shelter. She's already settled in nicely.",
            "The window blinds are stuck again. Need to replace the pull cord mechanism.",
            "Signed up for a woodworking class to build a coffee table from reclaimed lumber.",
            "The train station renovations are finally complete after eighteen months.",
            "Found a great podcast about the history of space exploration.",
            "The community pool added a lap lane for morning swimmers before work.",
            "Brought homemade cookies to the office potluck. They disappeared in minutes.",
            "The garden hose sprung a leak near the connector. Replacing it this weekend.",
            "Ran into an old college friend at the farmers market completely by surprise.",
            "The annual neighborhood garage sale is this Saturday from eight to two.",
            "Fixed the squeaky door hinge with some lubricant. Took about thirty seconds.",
            "The sunset cruise around the harbor was a great birthday celebration idea.",
            "Started composting kitchen scraps in the backyard tumbler composter.",
            "The library added a three-D printer that anyone can use with a reservation.",
            "Drove past the old high school and noticed they repainted the gymnasium.",
            "The weekend brunch spot has a new avocado toast that's actually worth the price.",
            "Replaced the shower curtain with a glass door panel for a cleaner look.",
            "The charity auction raised over two thousand dollars for the local food bank.",
            "Spent the afternoon at the beach collecting shells and skipping stones.",
            "The air filter in the HVAC system needs replacing every three months.",
            "Went kayaking on the river for the first time. Saw a great blue heron.",
            "The barbershop on fifth avenue still gives the best haircuts in town.",
            "Organized the garage tools on a new pegboard system from the hardware store.",
            "The county fair starts next Friday with rides, games, and livestock judging.",
            "Tried the new board game cafe downtown. They have over three hundred titles.",
            "The delivery driver left the package in the rain. Fortunately nothing was damaged.",
            "Started growing herbs on the kitchen windowsill. Basil and cilantro so far.",
            "The fireworks display for the holiday was the best one in years.",
            "Refinished the hardwood floors in the hallway. They look brand new.",
        ] {
            push_msg(&mut docs, text, "/corpus/bulk/noise");
        }

        Self { docs }
    }

    /// Inserts documents into the database (FTS only, no embeddings).
    pub fn insert_into(&self, db: &mut Database) {
        db.insert_documents(&self.docs)
            .expect("Failed to insert corpus documents");
    }

    /// Inserts documents and generates/inserts embeddings via the model.
    pub fn insert_with_embeddings(&self, db: &mut Database) {
        self.insert_into(db);

        let embedder = glhf::embed::Embedder::new().expect("Failed to create embedder");
        let contents: Vec<String> = self.docs.iter().map(|d| d.content.clone()).collect();
        let embeddings = embedder
            .embed_documents(&contents)
            .expect("Failed to generate embeddings");

        let pairs: Vec<(&str, &[f32])> = self
            .docs
            .iter()
            .zip(embeddings.iter())
            .map(|(d, e)| (d.id.as_str(), e.as_slice()))
            .collect();
        db.insert_embeddings(&pairs)
            .expect("Failed to insert embeddings");
    }
}

// ── Assertion helpers ──────────────────────────────────────────────────

/// Asserts that a result containing `needle` appears in the top `k` results.
#[track_caller]
pub fn assert_in_top_k(results: &[SearchResult], needle: &str, k: usize) {
    let found = results
        .iter()
        .take(k)
        .any(|r| r.content.to_lowercase().contains(&needle.to_lowercase()));
    assert!(
        found,
        "Expected doc containing '{needle}' in top {k}, got:\n{}",
        format_top_results(results, k)
    );
}

/// Asserts that the result containing `above` ranks higher than the one containing `below`.
#[track_caller]
pub fn assert_ranks_above(results: &[SearchResult], above: &str, below: &str) {
    let above_lower = above.to_lowercase();
    let below_lower = below.to_lowercase();
    let above_pos = results
        .iter()
        .position(|r| r.content.to_lowercase().contains(&above_lower));
    let below_pos = results
        .iter()
        .position(|r| r.content.to_lowercase().contains(&below_lower));
    match (above_pos, below_pos) {
        (Some(a), Some(b)) => assert!(
            a < b,
            "'{above}' at position {a} should rank above '{below}' at position {b}"
        ),
        (Some(_), None) => {} // above found, below not — that's fine
        (None, Some(b)) => {
            panic!("'{above}' not found in results, but '{below}' is at position {b}")
        }
        (None, None) => panic!("Neither '{above}' nor '{below}' found in results"),
    }
}

/// Asserts that NO result containing `needle` appears in the top `k` results.
#[track_caller]
pub fn assert_not_in_top_k(results: &[SearchResult], needle: &str, k: usize) {
    let found = results
        .iter()
        .take(k)
        .any(|r| r.content.to_lowercase().contains(&needle.to_lowercase()));
    assert!(
        !found,
        "Expected doc containing '{needle}' NOT in top {k}, but it was found"
    );
}

/// Formats the top results for diagnostic output.
fn format_top_results(results: &[SearchResult], k: usize) -> String {
    results
        .iter()
        .take(k)
        .enumerate()
        .map(|(i, r)| {
            let snippet: String = r.content.chars().take(80).collect();
            format!("  #{}: [{}] {snippet}...", i + 1, r.chunk_kind)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Creates a zero-filled embedding vector for edge case tests.
pub fn zero_embedding() -> Vec<f32> {
    vec![0.0_f32; EMBEDDING_DIM]
}
