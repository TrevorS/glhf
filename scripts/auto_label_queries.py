#!/usr/bin/env python3
"""Automatically label queries with relevant documents using heuristics.

Usage:
    python scripts/auto_label_queries.py [--candidates PATH] [--corpus PATH] [--output PATH]
"""

import argparse
import json
import math
import re
from collections import defaultdict
from pathlib import Path


def load_json(path: Path) -> dict:
    """Load a JSON file."""
    with open(path, 'r', encoding='utf-8') as f:
        return json.load(f)


def save_json(data: dict, path: Path) -> None:
    """Save data to JSON file."""
    with open(path, 'w', encoding='utf-8') as f:
        json.dump(data, f, indent=2)


def tokenize(text: str) -> list[str]:
    """Simple tokenization for BM25."""
    text = text.lower()
    text = re.sub(r'[^\w\s-]', ' ', text)
    return [w for w in text.split() if len(w) >= 2]


class BM25:
    """Simple BM25 implementation for relevance scoring."""

    def __init__(self, documents: list[dict], k1: float = 1.5, b: float = 0.75):
        self.k1 = k1
        self.b = b
        self.docs = documents
        self.doc_count = len(documents)

        # Tokenize documents
        self.doc_tokens = [tokenize(d.get('content', '')) for d in documents]
        self.doc_lengths = [len(tokens) for tokens in self.doc_tokens]
        self.avg_doc_length = sum(self.doc_lengths) / max(1, len(self.doc_lengths))

        # Build inverted index for document frequency
        self.doc_freq = defaultdict(int)
        for tokens in self.doc_tokens:
            for token in set(tokens):
                self.doc_freq[token] += 1

    def score(self, query: str, doc_idx: int) -> float:
        """Calculate BM25 score for a document given a query."""
        query_tokens = tokenize(query)
        doc_tokens = self.doc_tokens[doc_idx]
        doc_length = self.doc_lengths[doc_idx]

        # Term frequency in document
        tf = defaultdict(int)
        for token in doc_tokens:
            tf[token] += 1

        score = 0.0
        for term in query_tokens:
            if term not in tf:
                continue

            # IDF
            df = self.doc_freq.get(term, 0)
            idf = math.log((self.doc_count - df + 0.5) / (df + 0.5) + 1.0)

            # TF component with length normalization
            term_freq = tf[term]
            tf_component = (term_freq * (self.k1 + 1)) / (
                term_freq + self.k1 * (1 - self.b + self.b * doc_length / self.avg_doc_length)
            )

            score += idf * tf_component

        return score


def label_query(query: dict, corpus: list[dict], bm25: BM25) -> list[str]:
    """
    Return list of relevant doc IDs for this query.

    Uses BM25 + type-specific heuristics.
    """
    query_text = query['query'].lower()
    query_type = query['query_type']
    query_tokens = set(tokenize(query_text))

    scored = []

    for i, doc in enumerate(corpus):
        content = doc.get('content', '').lower()
        doc_tokens = set(tokenize(content))

        # Base BM25 score
        bm25_score = bm25.score(query_text, i)

        # Exact substring match bonus
        exact_match = 1.0 if query_text in content else 0.0

        # Token overlap ratio
        if query_tokens:
            overlap = len(query_tokens & doc_tokens) / len(query_tokens)
        else:
            overlap = 0.0

        # Type-specific bonuses
        type_bonus = 0.0

        if query_type == 'keyword':
            # For keyword queries, exact match is critical
            if exact_match > 0:
                type_bonus = 20.0
            elif overlap >= 0.8:
                type_bonus = 10.0

        elif query_type == 'tool':
            # For tool queries, match tool names and commands
            tool_name = doc.get('tool_name', '').lower()
            chunk_kind = doc.get('chunk_kind', '')

            # Check for tool name match
            tool_patterns = ['bash', 'git', 'cargo', 'read', 'edit', 'write', 'grep', 'npm', 'pip']
            for pattern in tool_patterns:
                if pattern in query_text and pattern in content:
                    type_bonus += 5.0
                if pattern in query_text and tool_name == pattern:
                    type_bonus += 10.0

            # Boost tool_use chunks for tool queries
            if chunk_kind == 'tool_use':
                type_bonus += 3.0

        elif query_type == 'error':
            # For error queries, boost error results
            is_error = doc.get('is_error', False)
            if is_error:
                type_bonus += 10.0
                # Extra boost if error terms overlap
                error_terms = {'error', 'fail', 'failed', 'exception', 'panic', 'crash'}
                if query_tokens & error_terms and doc_tokens & error_terms:
                    type_bonus += 5.0

        elif query_type == 'code':
            # For code queries, boost implementation-related content
            code_terms = {'impl', 'implement', 'struct', 'function', 'fn', 'def', 'class', 'parse', 'handle'}
            if query_tokens & code_terms:
                if doc_tokens & code_terms:
                    type_bonus += 5.0

            # Boost tool_use with code-like content
            if doc.get('chunk_kind') == 'tool_use':
                tool_name = doc.get('tool_name', '').lower()
                if tool_name in ('edit', 'write'):
                    type_bonus += 5.0

        elif query_type == 'semantic':
            # For semantic queries, rely more on BM25 but boost conceptual matches
            concept_terms = {'how', 'why', 'what', 'when', 'workflow', 'setup', 'configure', 'debug'}
            if query_tokens & concept_terms:
                # Boost assistant messages for conceptual queries
                if doc.get('role') == 'assistant':
                    type_bonus += 3.0

        # Combined score
        final_score = bm25_score * 2.0 + exact_match * 15.0 + overlap * 5.0 + type_bonus

        scored.append((doc['id'], final_score, bm25_score, exact_match, overlap, type_bonus))

    # Sort by final score
    scored.sort(key=lambda x: -x[1])

    # Return docs that pass threshold
    # For keyword/exact queries, require higher threshold
    if query_type == 'keyword':
        threshold = 15.0
    elif query_type in ('tool', 'error'):
        threshold = 8.0
    else:
        threshold = 5.0

    relevant = []
    for doc_id, score, bm25_score, exact, overlap, type_bonus in scored[:10]:
        if score >= threshold:
            relevant.append(doc_id)
            if len(relevant) >= 3:
                break

    return relevant


def main():
    parser = argparse.ArgumentParser(description='Auto-label queries')
    parser.add_argument('--candidates', '-c',
                        default='tests/eval_data/query_candidates.json',
                        help='Input query candidates file')
    parser.add_argument('--corpus', '-C',
                        default='tests/eval_data/corpus_expanded.json',
                        help='Corpus file')
    parser.add_argument('--output', '-o',
                        default='tests/eval_data/queries_expanded.json',
                        help='Output file for labeled queries')
    parser.add_argument('--min-relevant', type=int, default=1,
                        help='Minimum relevant docs per query')
    parser.add_argument('--verbose', '-v', action='store_true',
                        help='Show detailed output')
    args = parser.parse_args()

    # Load data
    print(f"Loading candidates from {args.candidates}...")
    candidates = load_json(Path(args.candidates))
    queries = candidates.get('queries', [])

    print(f"Loading corpus from {args.corpus}...")
    corpus = load_json(Path(args.corpus))
    documents = corpus.get('documents', [])

    print(f"Loaded {len(queries)} queries, {len(documents)} documents")

    # Build BM25 index
    print("Building BM25 index...")
    bm25 = BM25(documents)

    # Label each query
    print("\nLabeling queries...")
    labeled = []
    by_type = defaultdict(lambda: {'total': 0, 'labeled': 0})

    for query in queries:
        qtype = query['query_type']
        by_type[qtype]['total'] += 1

        relevant_ids = label_query(query, documents, bm25)

        if len(relevant_ids) >= args.min_relevant:
            query['relevant_doc_ids'] = relevant_ids
            labeled.append(query)
            by_type[qtype]['labeled'] += 1

            if args.verbose:
                print(f"  {query['id']}: \"{query['query'][:40]}...\" -> {len(relevant_ids)} relevant")
        elif args.verbose:
            print(f"  {query['id']}: \"{query['query'][:40]}...\" -> SKIPPED (no matches)")

    # Re-number IDs
    for i, q in enumerate(labeled, 1):
        q['id'] = f'q{i:03d}'

    # Save
    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    save_json({'queries': labeled}, output_path)

    # Summary
    print(f"\n{'=' * 60}")
    print(f"Labeled {len(labeled)} queries (from {len(queries)} candidates)")
    print(f"Saved to {output_path}")

    print("\nBy query type:")
    for qtype in sorted(by_type.keys()):
        stats = by_type[qtype]
        print(f"  {qtype}: {stats['labeled']}/{stats['total']} labeled")

    # Stats on relevance
    relevance_counts = [len(q['relevant_doc_ids']) for q in labeled]
    if relevance_counts:
        avg_relevant = sum(relevance_counts) / len(relevance_counts)
        print(f"\nRelevant docs per query: avg={avg_relevant:.1f}, min={min(relevance_counts)}, max={max(relevance_counts)}")


if __name__ == '__main__':
    main()
