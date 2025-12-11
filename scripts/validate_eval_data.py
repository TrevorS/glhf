#!/usr/bin/env python3
"""Validate evaluation data quality.

Usage:
    python scripts/validate_eval_data.py [--corpus PATH] [--queries PATH]
"""

import argparse
import json
from collections import defaultdict
from pathlib import Path


def load_json(path: Path) -> dict:
    """Load a JSON file."""
    with open(path, 'r', encoding='utf-8') as f:
        return json.load(f)


def validate_corpus(corpus: dict) -> list[str]:
    """Validate corpus data."""
    errors = []
    documents = corpus.get('documents', [])

    if not documents:
        errors.append("Corpus is empty")
        return errors

    # Check for duplicate IDs
    ids = [d['id'] for d in documents]
    if len(ids) != len(set(ids)):
        dup_count = len(ids) - len(set(ids))
        errors.append(f"Corpus has {dup_count} duplicate document IDs")

    # Check each document
    for i, doc in enumerate(documents):
        if not doc.get('id'):
            errors.append(f"Document {i} missing 'id' field")
        if not doc.get('content'):
            errors.append(f"Document {doc.get('id', i)} has empty content")
        if not doc.get('chunk_kind'):
            errors.append(f"Document {doc.get('id', i)} missing 'chunk_kind'")

    return errors


def validate_queries(queries: dict, corpus_ids: set) -> list[str]:
    """Validate queries data against corpus."""
    errors = []
    query_list = queries.get('queries', [])

    if not query_list:
        errors.append("No queries found")
        return errors

    # Check for duplicate query IDs
    ids = [q['id'] for q in query_list]
    if len(ids) != len(set(ids)):
        dup_count = len(ids) - len(set(ids))
        errors.append(f"Queries have {dup_count} duplicate query IDs")

    # Check for duplicate query text
    texts = [q['query'] for q in query_list]
    if len(texts) != len(set(texts)):
        dup_count = len(texts) - len(set(texts))
        errors.append(f"Queries have {dup_count} duplicate query texts")

    # Check each query
    for q in query_list:
        qid = q.get('id', 'unknown')

        if not q.get('query'):
            errors.append(f"Query {qid} has empty query text")

        if not q.get('query_type'):
            errors.append(f"Query {qid} missing 'query_type'")

        relevant = q.get('relevant_doc_ids', [])
        if not relevant:
            errors.append(f"Query {qid} has no relevant documents")
        else:
            # Check all relevant docs exist in corpus
            for doc_id in relevant:
                if doc_id not in corpus_ids:
                    errors.append(f"Query {qid}: relevant doc {doc_id} not in corpus")

    return errors


def compute_statistics(corpus: dict, queries: dict) -> dict:
    """Compute statistics about the eval data."""
    documents = corpus.get('documents', [])
    query_list = queries.get('queries', [])

    # Corpus stats
    by_kind = defaultdict(int)
    by_tool = defaultdict(int)
    error_count = 0
    content_lengths = []

    for doc in documents:
        by_kind[doc.get('chunk_kind', 'unknown')] += 1
        if doc.get('tool_name'):
            by_tool[doc['tool_name']] += 1
        if doc.get('is_error'):
            error_count += 1
        content_lengths.append(len(doc.get('content', '')))

    # Query stats
    by_type = defaultdict(int)
    relevance_counts = []

    for q in query_list:
        by_type[q.get('query_type', 'unknown')] += 1
        relevance_counts.append(len(q.get('relevant_doc_ids', [])))

    return {
        'corpus': {
            'total': len(documents),
            'by_kind': dict(by_kind),
            'by_tool': dict(by_tool),
            'errors': error_count,
            'avg_content_length': sum(content_lengths) / max(1, len(content_lengths)),
            'min_content_length': min(content_lengths) if content_lengths else 0,
            'max_content_length': max(content_lengths) if content_lengths else 0,
        },
        'queries': {
            'total': len(query_list),
            'by_type': dict(by_type),
            'avg_relevant_docs': sum(relevance_counts) / max(1, len(relevance_counts)),
            'min_relevant_docs': min(relevance_counts) if relevance_counts else 0,
            'max_relevant_docs': max(relevance_counts) if relevance_counts else 0,
        },
    }


def main():
    parser = argparse.ArgumentParser(description='Validate eval data')
    parser.add_argument('--corpus', '-c',
                        default='tests/eval_data/corpus_expanded.json',
                        help='Corpus file path')
    parser.add_argument('--queries', '-q',
                        default='tests/eval_data/queries_expanded.json',
                        help='Queries file path')
    args = parser.parse_args()

    corpus_path = Path(args.corpus)
    queries_path = Path(args.queries)

    print("=" * 60)
    print("Eval Data Validation")
    print("=" * 60)

    # Load corpus
    if not corpus_path.exists():
        print(f"\n❌ Corpus file not found: {corpus_path}")
        return 1

    print(f"\nLoading corpus from {corpus_path}...")
    corpus = load_json(corpus_path)

    # Validate corpus
    corpus_errors = validate_corpus(corpus)
    if corpus_errors:
        print(f"\n❌ Corpus validation errors ({len(corpus_errors)}):")
        for err in corpus_errors[:10]:  # Show first 10
            print(f"   - {err}")
        if len(corpus_errors) > 10:
            print(f"   ... and {len(corpus_errors) - 10} more")
    else:
        print("✓ Corpus validation passed")

    # Load queries
    if not queries_path.exists():
        print(f"\n⚠ Queries file not found: {queries_path}")
        print("  Run scripts/label_queries.py to create labeled queries")
    else:
        print(f"\nLoading queries from {queries_path}...")
        queries = load_json(queries_path)

        # Validate queries
        corpus_ids = {d['id'] for d in corpus.get('documents', [])}
        query_errors = validate_queries(queries, corpus_ids)

        if query_errors:
            print(f"\n❌ Query validation errors ({len(query_errors)}):")
            for err in query_errors[:10]:
                print(f"   - {err}")
            if len(query_errors) > 10:
                print(f"   ... and {len(query_errors) - 10} more")
        else:
            print("✓ Query validation passed")

        # Compute and display statistics
        stats = compute_statistics(corpus, queries)

        print("\n" + "-" * 60)
        print("Statistics")
        print("-" * 60)

        print(f"\nCorpus ({stats['corpus']['total']} documents):")
        print("  By chunk kind:")
        for kind, count in sorted(stats['corpus']['by_kind'].items()):
            print(f"    {kind}: {count}")

        if stats['corpus']['by_tool']:
            print("  By tool (top 10):")
            sorted_tools = sorted(stats['corpus']['by_tool'].items(), key=lambda x: -x[1])
            for tool, count in sorted_tools[:10]:
                print(f"    {tool}: {count}")

        print(f"  Error results: {stats['corpus']['errors']}")
        print(f"  Content length: avg={stats['corpus']['avg_content_length']:.0f}, "
              f"min={stats['corpus']['min_content_length']}, "
              f"max={stats['corpus']['max_content_length']}")

        print(f"\nQueries ({stats['queries']['total']} queries):")
        print("  By type:")
        for qtype, count in sorted(stats['queries']['by_type'].items()):
            print(f"    {qtype}: {count}")

        print(f"  Relevant docs per query: avg={stats['queries']['avg_relevant_docs']:.1f}, "
              f"min={stats['queries']['min_relevant_docs']}, "
              f"max={stats['queries']['max_relevant_docs']}")

    print("\n" + "=" * 60)
    all_passed = not corpus_errors and (not queries_path.exists() or not query_errors)
    if all_passed:
        print("✓ All validations passed!")
    else:
        print("❌ Validation failed - please fix errors above")

    return 0 if all_passed else 1


if __name__ == '__main__':
    exit(main())
