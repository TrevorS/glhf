#!/usr/bin/env python3
"""Interactive tool for labeling query-document relevance.

Usage:
    python scripts/label_queries.py [--candidates PATH] [--output PATH]

Commands during labeling:
    y       - Mark as relevant
    n       - Mark as not relevant
    s       - Skip this query entirely
    d       - Done with this query (move to next)
    q       - Quit and save progress
    ?       - Show help
"""

import argparse
import json
import subprocess
import sys
from pathlib import Path


def load_json(path: Path) -> dict:
    """Load a JSON file."""
    with open(path, 'r', encoding='utf-8') as f:
        return json.load(f)


def save_json(data: dict, path: Path) -> None:
    """Save data to JSON file."""
    with open(path, 'w', encoding='utf-8') as f:
        json.dump(data, f, indent=2)


def search_corpus(query: str, corpus_path: str, limit: int = 10) -> list[dict]:
    """Search corpus using glhf and return results."""
    try:
        # Try to use glhf for searching - fall back to simple matching
        result = subprocess.run(
            ['./target/release/glhf', 'search', query, '-n', str(limit)],
            capture_output=True,
            text=True,
            timeout=10,
        )
        if result.returncode == 0:
            # Parse glhf output - just return raw output for display
            return [{'output': result.stdout}]
    except Exception:
        pass

    # Fallback: simple text search in corpus
    corpus = load_json(Path(corpus_path))
    query_lower = query.lower()
    matches = []

    for doc in corpus['documents']:
        content = doc.get('content', '').lower()
        if query_lower in content:
            score = content.count(query_lower) / max(1, len(content) / 100)
            matches.append({
                'id': doc['id'],
                'content': doc['content'][:300],
                'chunk_kind': doc.get('chunk_kind', 'unknown'),
                'score': score,
            })

    # Sort by score and limit
    matches.sort(key=lambda x: -x['score'])
    return matches[:limit]


def truncate(text: str, max_len: int = 200) -> str:
    """Truncate text to max length."""
    text = ' '.join(text.split())  # Normalize whitespace
    if len(text) <= max_len:
        return text
    return text[:max_len - 3] + '...'


def display_query(query: dict, num: int, total: int) -> None:
    """Display query information."""
    print("\n" + "=" * 60)
    print(f"Query [{num}/{total}] - Type: {query['query_type']}")
    print(f"Query: \"{query['query']}\"")
    if query.get('relevant_doc_ids'):
        print(f"Already labeled: {len(query['relevant_doc_ids'])} relevant docs")
    print("=" * 60)


def display_results(results: list[dict], corpus: dict) -> list[str]:
    """Display search results and return doc IDs."""
    doc_ids = []

    # If we got glhf output, just print it
    if results and 'output' in results[0]:
        print(results[0]['output'])
        return []

    # Otherwise, display our corpus matches
    print(f"\nTop matches from corpus ({len(results)} results):\n")

    for i, result in enumerate(results, 1):
        doc_id = result['id']
        doc_ids.append(doc_id)

        kind = result.get('chunk_kind', 'unknown')
        content = truncate(result.get('content', ''), 150)
        score = result.get('score', 0)

        print(f"  [{i}] ({kind}) [{doc_id[:8]}] score={score:.3f}")
        print(f"      \"{content}\"")
        print()

    return doc_ids


def label_query(query: dict, corpus: dict, corpus_path: str) -> tuple[dict, bool]:
    """
    Interactively label a single query.

    Returns (updated_query, should_continue).
    """
    # Search for relevant documents
    results = search_corpus(query['query'], corpus_path)
    doc_ids = display_results(results, corpus)

    # If no doc_ids, we're using glhf output - need to look up manually
    if not doc_ids:
        print("\nEnter doc IDs to mark as relevant (comma-separated), or:")
        print("  s = skip, d = done, q = quit")
    else:
        print("\nMark relevant docs by number (e.g., '1,3,5'), or:")
        print("  a = all shown are relevant")
        print("  s = skip this query")
        print("  d = done with this query")
        print("  q = quit and save")

    relevant_ids = set(query.get('relevant_doc_ids', []))

    while True:
        try:
            user_input = input("\n> ").strip().lower()
        except (EOFError, KeyboardInterrupt):
            print("\nSaving and exiting...")
            query['relevant_doc_ids'] = list(relevant_ids)
            return query, False

        if user_input == 'q':
            query['relevant_doc_ids'] = list(relevant_ids)
            return query, False

        if user_input == 's':
            # Skip this query - don't include in output
            return None, True

        if user_input == 'd':
            query['relevant_doc_ids'] = list(relevant_ids)
            return query, True

        if user_input == 'a' and doc_ids:
            for doc_id in doc_ids:
                relevant_ids.add(doc_id)
            print(f"Marked all {len(doc_ids)} docs as relevant.")
            query['relevant_doc_ids'] = list(relevant_ids)
            return query, True

        if user_input == '?':
            print("\nCommands:")
            print("  1,2,3  - Mark docs by number as relevant")
            print("  a      - Mark all shown as relevant")
            print("  s      - Skip this query")
            print("  d      - Done, move to next query")
            print("  q      - Quit and save progress")
            continue

        # Try to parse as numbers
        if doc_ids:
            try:
                nums = [int(x.strip()) for x in user_input.split(',') if x.strip()]
                for num in nums:
                    if 1 <= num <= len(doc_ids):
                        doc_id = doc_ids[num - 1]
                        relevant_ids.add(doc_id)
                        print(f"  Added: {doc_id[:8]}...")
                    else:
                        print(f"  Invalid number: {num}")
            except ValueError:
                print("Invalid input. Enter numbers separated by commas, or a command.")
        else:
            # Manual doc ID entry
            if user_input:
                for doc_id in user_input.split(','):
                    doc_id = doc_id.strip()
                    if doc_id:
                        relevant_ids.add(doc_id)
                        print(f"  Added: {doc_id[:8]}...")


def main():
    parser = argparse.ArgumentParser(description='Label query-document relevance')
    parser.add_argument('--candidates', '-c',
                        default='tests/eval_data/query_candidates.json',
                        help='Input query candidates file')
    parser.add_argument('--corpus', '-C',
                        default='tests/eval_data/corpus_expanded.json',
                        help='Corpus file for searching')
    parser.add_argument('--output', '-o',
                        default='tests/eval_data/queries_expanded.json',
                        help='Output file for labeled queries')
    parser.add_argument('--start', '-s', type=int, default=0,
                        help='Start from query index (for resuming)')
    parser.add_argument('--limit', '-l', type=int, default=100,
                        help='Maximum queries to label')
    args = parser.parse_args()

    # Load data
    candidates_path = Path(args.candidates)
    corpus_path = Path(args.corpus)
    output_path = Path(args.output)

    print(f"Loading candidates from {candidates_path}...")
    candidates = load_json(candidates_path)
    queries = candidates.get('queries', [])

    print(f"Loading corpus from {corpus_path}...")
    corpus = load_json(corpus_path)

    # Load existing progress if any
    labeled = []
    if output_path.exists():
        existing = load_json(output_path)
        labeled = existing.get('queries', [])
        labeled_ids = {q['id'] for q in labeled}
        # Filter out already-labeled queries
        queries = [q for q in queries if q['id'] not in labeled_ids]
        print(f"Resuming: {len(labeled)} queries already labeled, {len(queries)} remaining")

    print(f"\nWill label up to {args.limit} queries starting from index {args.start}")
    print("Commands: y=relevant, n=not relevant, s=skip, d=done, q=quit, ?=help\n")

    # Process queries
    queries_to_label = queries[args.start:args.start + args.limit]
    total = len(queries_to_label)

    for i, query in enumerate(queries_to_label, 1):
        display_query(query, i + len(labeled), total + len(labeled))

        updated_query, should_continue = label_query(query, corpus, str(corpus_path))

        if updated_query:
            if updated_query.get('relevant_doc_ids'):
                labeled.append(updated_query)
                print(f"\n✓ Saved query with {len(updated_query['relevant_doc_ids'])} relevant docs")
            else:
                print("\n⚠ Query has no relevant docs - skipping")

        if not should_continue:
            break

    # Save results
    output_path.parent.mkdir(parents=True, exist_ok=True)
    save_json({'queries': labeled}, output_path)

    print(f"\n{'=' * 60}")
    print(f"Saved {len(labeled)} labeled queries to {output_path}")

    # Stats
    by_type = {}
    for q in labeled:
        qtype = q.get('query_type', 'unknown')
        by_type[qtype] = by_type.get(qtype, 0) + 1

    print("\nDistribution by type:")
    for qtype, count in sorted(by_type.items()):
        print(f"  {qtype}: {count}")


if __name__ == '__main__':
    main()
