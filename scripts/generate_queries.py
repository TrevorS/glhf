#!/usr/bin/env python3
"""Generate query candidates from corpus for manual labeling.

Usage:
    python scripts/generate_queries.py [--corpus PATH] [--output PATH]
"""

import argparse
import json
import re
from collections import defaultdict
from pathlib import Path


# Query templates by type
QUERY_TEMPLATES = {
    'semantic': [
        # Workflow queries
        "how to {verb} {noun}",
        "{noun} workflow",
        "setup {noun}",
        "configure {noun}",
        "debug {noun} issues",
        "fix {noun} problems",
        "improve {noun} performance",
        # Conceptual
        "what is {noun}",
        "explain {noun}",
        "understand {noun}",
    ],
    'tool': [
        # Git operations
        "git commit {noun}",
        "git push {noun}",
        "git log {noun}",
        "git diff {noun}",
        "git checkout {noun}",
        "git merge {noun}",
        # Build tools
        "cargo build {noun}",
        "cargo test {noun}",
        "npm install {noun}",
        "pip install {noun}",
        # File operations
        "read file {noun}",
        "edit {noun}",
        "write to {noun}",
        "grep {noun}",
        "find {noun}",
        # Commands
        "run {noun} command",
        "execute {noun}",
        "curl {noun}",
    ],
    'code': [
        # Implementation
        "implement {noun}",
        "create {noun} function",
        "add {noun} to code",
        "define {noun} struct",
        "write {noun} test",
        # Patterns
        "{noun} pattern",
        "{noun} implementation",
        "parse {noun}",
        "serialize {noun}",
        "handle {noun}",
    ],
    'keyword': [
        # Exact matches
        "{noun}",
        "{noun} {noun2}",
    ],
    'error': [
        # Error patterns
        "{noun} error",
        "failed to {verb}",
        "{noun} failure",
        "cannot {verb} {noun}",
        "error {noun}",
        "{noun} not found",
        "invalid {noun}",
    ],
}

# Common verbs and nouns extracted from typical Claude Code usage
VERBS = [
    'build', 'run', 'test', 'deploy', 'install', 'configure', 'setup',
    'create', 'delete', 'update', 'fix', 'debug', 'compile', 'parse',
    'handle', 'process', 'validate', 'check', 'verify', 'implement',
    'add', 'remove', 'modify', 'refactor', 'optimize', 'search', 'find',
]

NOUNS = [
    'error', 'test', 'code', 'file', 'function', 'struct', 'module',
    'package', 'dependency', 'config', 'settings', 'database', 'api',
    'endpoint', 'request', 'response', 'authentication', 'authorization',
    'user', 'session', 'token', 'cache', 'memory', 'performance',
]


def load_corpus(path: Path) -> dict:
    """Load the corpus JSON file."""
    with open(path, 'r', encoding='utf-8') as f:
        return json.load(f)


def extract_keywords(content: str) -> set[str]:
    """Extract potential keywords from content."""
    # Remove common noise
    content = re.sub(r'[^\w\s-]', ' ', content.lower())
    words = content.split()

    # Filter to interesting words
    keywords = set()
    for word in words:
        if len(word) >= 3 and word not in STOP_WORDS:
            keywords.add(word)

    return keywords


STOP_WORDS = {
    'the', 'and', 'for', 'not', 'you', 'this', 'that', 'with', 'are',
    'from', 'your', 'have', 'has', 'had', 'was', 'were', 'been', 'being',
    'will', 'would', 'could', 'should', 'may', 'might', 'must', 'can',
    'let', 'lets', 'use', 'using', 'used', 'make', 'made', 'get', 'got',
    'just', 'now', 'here', 'there', 'when', 'where', 'what', 'which',
    'who', 'how', 'why', 'all', 'each', 'any', 'some', 'such', 'only',
    'other', 'than', 'then', 'also', 'more', 'most', 'out', 'but', 'yet',
    'about', 'into', 'over', 'after', 'before', 'between', 'under',
    'true', 'false', 'null', 'none', 'yes', 'type', 'text', 'content',
}


def extract_tool_terms(corpus: dict) -> dict[str, set[str]]:
    """Extract tool-specific terms from the corpus."""
    tool_terms = defaultdict(set)

    for doc in corpus['documents']:
        content = doc.get('content', '')
        kind = doc.get('chunk_kind', '')
        tool = doc.get('tool_name', '')

        if kind == 'tool_use' and tool:
            keywords = extract_keywords(content)
            tool_terms[tool].update(keywords)

    return tool_terms


def extract_interesting_docs(corpus: dict, limit_per_kind: int = 50) -> list[dict]:
    """Extract interesting documents for query generation."""
    by_kind = defaultdict(list)

    for doc in corpus['documents']:
        kind = doc.get('chunk_kind', '')
        content = doc.get('content', '')

        # Skip very short or very long content
        if len(content) < 20 or len(content) > 500:
            continue

        by_kind[kind].append(doc)

    # Sample from each kind
    interesting = []
    for kind, docs in by_kind.items():
        # Sort by content length (medium-length docs are often most interesting)
        docs.sort(key=lambda d: abs(len(d['content']) - 150))
        interesting.extend(docs[:limit_per_kind])

    return interesting


def generate_query_candidates(corpus: dict) -> list[dict]:
    """Generate query candidates from corpus."""
    candidates = []
    query_id = 1

    # Extract keywords from corpus
    all_keywords = set()
    for doc in corpus['documents']:
        all_keywords.update(extract_keywords(doc.get('content', '')))

    # Filter to most relevant keywords (those that appear multiple times)
    keyword_counts = defaultdict(int)
    for doc in corpus['documents']:
        for kw in extract_keywords(doc.get('content', '')):
            keyword_counts[kw] += 1

    # Top keywords by frequency (but not too common)
    top_keywords = [
        kw for kw, count in sorted(keyword_counts.items(), key=lambda x: -x[1])
        if 3 <= count <= 50  # Not too rare, not too common
    ][:200]

    # Target: 25 semantic, 30 tool, 20 code, 15 keyword, 10 error = 100 total
    # Generate in rounds to ensure diversity

    # Generate keyword queries first (using actual corpus terms) - target 15
    for noun in top_keywords[:20]:
        candidates.append({
            'id': f'q{query_id:03d}',
            'query': noun,
            'query_type': 'keyword',
            'relevant_doc_ids': [],
            'tags': [noun],
        })
        query_id += 1

    # Generate error queries - target 10
    error_docs = [d for d in corpus['documents'] if d.get('is_error')]
    error_keywords = set()
    for doc in error_docs[:50]:
        error_keywords.update(extract_keywords(doc.get('content', '')))

    error_keywords_list = list(error_keywords)[:15]
    for template in QUERY_TEMPLATES['error'][:2]:
        for noun in error_keywords_list[:8]:
            query_text = template.format(verb='build', noun=noun)
            if len(query_text) > 8:
                candidates.append({
                    'id': f'q{query_id:03d}',
                    'query': query_text,
                    'query_type': 'error',
                    'relevant_doc_ids': [],
                    'tags': [noun],
                })
                query_id += 1

    # Generate code queries - target 20
    for template in QUERY_TEMPLATES['code'][:4]:
        for noun in top_keywords[:8]:
            query_text = template.format(noun=noun)
            if len(query_text) > 8:
                candidates.append({
                    'id': f'q{query_id:03d}',
                    'query': query_text,
                    'query_type': 'code',
                    'relevant_doc_ids': [],
                    'tags': [noun],
                })
                query_id += 1

    # Generate tool queries - target 30
    for template in QUERY_TEMPLATES['tool'][:6]:
        for noun in top_keywords[:8]:
            query_text = template.format(noun=noun)
            if len(query_text) > 8:
                candidates.append({
                    'id': f'q{query_id:03d}',
                    'query': query_text,
                    'query_type': 'tool',
                    'relevant_doc_ids': [],
                    'tags': [noun],
                })
                query_id += 1

    # Generate semantic queries - target 25
    for template in QUERY_TEMPLATES['semantic'][:5]:
        for noun in top_keywords[:8]:
            query_text = template.format(verb='debug', noun=noun)
            if len(query_text) > 10:
                candidates.append({
                    'id': f'q{query_id:03d}',
                    'query': query_text,
                    'query_type': 'semantic',
                    'relevant_doc_ids': [],
                    'tags': [noun],
                })
                query_id += 1

    return candidates


def deduplicate_queries(candidates: list[dict]) -> list[dict]:
    """Remove duplicate or very similar queries."""
    seen = set()
    unique = []

    for q in candidates:
        # Normalize query
        normalized = ' '.join(q['query'].lower().split())
        if normalized not in seen and len(normalized) >= 8:
            seen.add(normalized)
            unique.append(q)

    return unique


def main():
    parser = argparse.ArgumentParser(description='Generate query candidates')
    parser.add_argument('--corpus', '-c',
                        default='tests/eval_data/corpus_expanded.json',
                        help='Input corpus file')
    parser.add_argument('--output', '-o',
                        default='tests/eval_data/query_candidates.json',
                        help='Output file for query candidates')
    parser.add_argument('--limit', '-l', type=int, default=500,
                        help='Maximum number of candidates to generate')
    args = parser.parse_args()

    # Load corpus
    corpus_path = Path(args.corpus)
    print(f"Loading corpus from {corpus_path}...")
    corpus = load_corpus(corpus_path)
    print(f"Loaded {len(corpus['documents'])} documents")

    # Generate candidates
    print("\nGenerating query candidates...")
    candidates = generate_query_candidates(corpus)
    print(f"Generated {len(candidates)} raw candidates")

    # Deduplicate
    candidates = deduplicate_queries(candidates)
    print(f"After deduplication: {len(candidates)} candidates")

    # Limit
    candidates = candidates[:args.limit]
    print(f"Limited to: {len(candidates)} candidates")

    # Re-number IDs
    for i, q in enumerate(candidates, 1):
        q['id'] = f'q{i:03d}'

    # Count by type
    by_type = defaultdict(int)
    for q in candidates:
        by_type[q['query_type']] += 1

    print("\nDistribution by type:")
    for qtype, count in sorted(by_type.items()):
        print(f"  {qtype}: {count}")

    # Save
    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    with open(output_path, 'w', encoding='utf-8') as f:
        json.dump({'queries': candidates}, f, indent=2)

    print(f"\nSaved {len(candidates)} query candidates to {output_path}")
    print("\nNext: Run scripts/label_queries.py to label relevant docs")


if __name__ == '__main__':
    main()
