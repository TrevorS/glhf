#!/usr/bin/env python3
"""Sample a diverse corpus from Claude Code conversation files.

Usage:
    python scripts/sample_corpus.py [--output PATH] [--size N]
"""

import argparse
import hashlib
import json
import os
import random
from collections import defaultdict
from pathlib import Path


def parse_jsonl_file(path: Path) -> list[dict]:
    """Parse a JSONL file and extract documents."""
    docs = []
    project = extract_project_name(path)

    try:
        with open(path, 'r', encoding='utf-8') as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    entry = json.loads(line)
                    doc = extract_document(entry, project)
                    if doc:
                        docs.append(doc)
                except json.JSONDecodeError:
                    continue
    except Exception as e:
        print(f"Warning: Failed to parse {path}: {e}")

    return docs


def extract_project_name(path: Path) -> str:
    """Extract project name from path like ~/.claude/projects/-Users-foo-bar/..."""
    parts = path.parts
    for i, part in enumerate(parts):
        if part == 'projects' and i + 1 < len(parts):
            encoded = parts[i + 1]
            # Decode: -Users-foo-bar -> /Users/foo/bar
            if encoded.startswith('-'):
                return encoded.replace('-', '/')
            return encoded
    return "unknown"


def generate_doc_id(content: str, chunk_kind: str) -> str:
    """Generate deterministic document ID."""
    data = f"{chunk_kind}:{content}"
    return hashlib.sha256(data.encode()).hexdigest()[:16]


def extract_document(entry: dict, project: str) -> dict | None:
    """Extract a document from a conversation entry."""
    entry_type = entry.get('type')

    if entry_type == 'user':
        # User message - can contain text OR tool_result
        message = entry.get('message', {})
        if isinstance(message, dict):
            content = message.get('content', '')

            # Check for tool_result blocks in content array
            if isinstance(content, list):
                for block in content:
                    if isinstance(block, dict) and block.get('type') == 'tool_result':
                        result_content = block.get('content', '')
                        is_error = block.get('is_error', False)

                        # Handle nested content arrays
                        if isinstance(result_content, list):
                            result_content = ' '.join(
                                item.get('text', '') if isinstance(item, dict) else str(item)
                                for item in result_content
                            )

                        if result_content and len(str(result_content)) >= 10:
                            return {
                                'id': generate_doc_id(str(result_content), 'tool_result'),
                                'content': str(result_content)[:2000],
                                'chunk_kind': 'tool_result',
                                'is_error': is_error,
                                'project': project,
                            }
                # If no tool_result, skip array content (it's not a simple message)
                return None
        else:
            content = str(message)

        if not content or len(content) < 10:
            return None

        return {
            'id': generate_doc_id(content, 'message'),
            'content': content[:2000],  # Truncate very long content
            'chunk_kind': 'message',
            'role': 'user',
            'project': project,
        }

    elif entry_type == 'assistant':
        # Assistant message
        message = entry.get('message', {})
        content_blocks = message.get('content', [])

        docs = []
        for block in content_blocks:
            if isinstance(block, dict):
                block_type = block.get('type')

                if block_type == 'text':
                    text = block.get('text', '')
                    if text and len(text) >= 10:
                        return {
                            'id': generate_doc_id(text, 'message'),
                            'content': text[:2000],
                            'chunk_kind': 'message',
                            'role': 'assistant',
                            'project': project,
                        }

                elif block_type == 'tool_use':
                    tool_name = block.get('name', 'unknown')
                    tool_input = block.get('input', {})

                    # Extract meaningful content from tool input
                    content = format_tool_use(tool_name, tool_input)
                    if content and len(content) >= 10:
                        return {
                            'id': generate_doc_id(content, 'tool_use'),
                            'content': content[:2000],
                            'chunk_kind': 'tool_use',
                            'tool_name': tool_name,
                            'project': project,
                        }

                elif block_type == 'tool_result':
                    tool_use_id = block.get('tool_use_id', '')
                    result_content = block.get('content', '')
                    is_error = block.get('is_error', False)

                    if isinstance(result_content, list):
                        result_content = ' '.join(
                            item.get('text', '') if isinstance(item, dict) else str(item)
                            for item in result_content
                        )

                    if result_content and len(result_content) >= 10:
                        return {
                            'id': generate_doc_id(result_content, 'tool_result'),
                            'content': result_content[:2000],
                            'chunk_kind': 'tool_result',
                            'is_error': is_error,
                            'project': project,
                        }

    return None


def format_tool_use(tool_name: str, tool_input: dict) -> str:
    """Format tool use as searchable content."""
    if tool_name == 'Bash':
        return tool_input.get('command', '')
    elif tool_name == 'Read':
        return f"Read file: {tool_input.get('file_path', '')}"
    elif tool_name == 'Write':
        path = tool_input.get('file_path', '')
        content = tool_input.get('content', '')[:500]
        return f"Write to {path}: {content}"
    elif tool_name == 'Edit':
        path = tool_input.get('file_path', '')
        old = tool_input.get('old_string', '')[:200]
        new = tool_input.get('new_string', '')[:200]
        return f"Edit {path}: replace '{old}' with '{new}'"
    elif tool_name == 'Grep':
        pattern = tool_input.get('pattern', '')
        path = tool_input.get('path', '')
        return f"Grep for '{pattern}' in {path}"
    elif tool_name == 'Glob':
        pattern = tool_input.get('pattern', '')
        return f"Glob: {pattern}"
    elif tool_name == 'Task':
        prompt = tool_input.get('prompt', '')[:500]
        return f"Task: {prompt}"
    elif tool_name == 'WebFetch':
        url = tool_input.get('url', '')
        return f"Fetch URL: {url}"
    elif tool_name == 'WebSearch':
        query = tool_input.get('query', '')
        return f"Web search: {query}"
    else:
        # Generic fallback
        return json.dumps(tool_input)[:500]


def discover_conversation_files(base_dir: Path) -> list[Path]:
    """Find all JSONL conversation files."""
    files = []
    for root, dirs, filenames in os.walk(base_dir):
        for filename in filenames:
            if filename.endswith('.jsonl'):
                files.append(Path(root) / filename)
    return files


def sample_corpus(
    all_docs: list[dict],
    target_size: int = 1000,
    seed: int = 42
) -> list[dict]:
    """Sample a diverse corpus from all documents."""
    random.seed(seed)

    # Group by chunk_kind and role/tool
    groups = defaultdict(list)
    for doc in all_docs:
        kind = doc['chunk_kind']
        if kind == 'message':
            role = doc.get('role', 'unknown')
            groups[f'message_{role}'].append(doc)
        elif kind == 'tool_use':
            tool = doc.get('tool_name', 'unknown')
            groups[f'tool_use_{tool}'].append(doc)
        elif kind == 'tool_result':
            is_error = doc.get('is_error', False)
            groups[f'tool_result_{"error" if is_error else "success"}'].append(doc)
        else:
            groups['other'].append(doc)

    print(f"\nDocument groups found:")
    for group, docs in sorted(groups.items()):
        print(f"  {group}: {len(docs)}")

    # Target distribution
    targets = {
        'message_user': 200,
        'message_assistant': 200,
        'tool_use_Bash': 80,
        'tool_use_Read': 50,
        'tool_use_Edit': 50,
        'tool_use_Grep': 30,
        'tool_use_Write': 30,
        'tool_use_Glob': 20,
        'tool_use_Task': 20,
        'tool_use_WebFetch': 10,
        'tool_use_WebSearch': 10,
        'tool_result_success': 200,
        'tool_result_error': 100,
    }

    sampled = []
    seen_ids = set()

    for group, target in targets.items():
        available = groups.get(group, [])
        if not available:
            print(f"  Warning: No docs for {group}")
            continue

        # Sample up to target, avoiding duplicates
        candidates = [d for d in available if d['id'] not in seen_ids]
        n = min(target, len(candidates))
        selected = random.sample(candidates, n)

        for doc in selected:
            seen_ids.add(doc['id'])
            sampled.append(doc)

        print(f"  Sampled {n}/{target} from {group}")

    # Fill remaining with random diverse docs
    remaining = target_size - len(sampled)
    if remaining > 0:
        all_remaining = [d for d in all_docs if d['id'] not in seen_ids]
        if all_remaining:
            extra = random.sample(all_remaining, min(remaining, len(all_remaining)))
            sampled.extend(extra)
            print(f"  Added {len(extra)} extra docs to reach target")

    # Shuffle final corpus
    random.shuffle(sampled)

    return sampled


def main():
    parser = argparse.ArgumentParser(description='Sample corpus from conversation files')
    parser.add_argument('--output', '-o',
                        default='tests/eval_data/corpus_expanded.json',
                        help='Output file path')
    parser.add_argument('--size', '-s', type=int, default=1000,
                        help='Target corpus size')
    parser.add_argument('--seed', type=int, default=42,
                        help='Random seed for reproducibility')
    args = parser.parse_args()

    # Find conversation files
    projects_dir = Path.home() / '.claude' / 'projects'
    print(f"Scanning {projects_dir}...")

    files = discover_conversation_files(projects_dir)
    print(f"Found {len(files)} conversation files")

    # Parse all documents
    print("Parsing documents...")
    all_docs = []
    for f in files:
        all_docs.extend(parse_jsonl_file(f))

    print(f"Total documents (raw): {len(all_docs)}")

    # Deduplicate by ID (same content can appear in multiple files)
    seen_ids = set()
    unique_docs = []
    for doc in all_docs:
        if doc['id'] not in seen_ids:
            seen_ids.add(doc['id'])
            unique_docs.append(doc)
    all_docs = unique_docs
    print(f"Total documents (unique): {len(all_docs)}")

    # Sample corpus
    print(f"\nSampling {args.size} documents...")
    corpus = sample_corpus(all_docs, target_size=args.size, seed=args.seed)

    # Remove project field for output (not needed in eval)
    for doc in corpus:
        doc.pop('project', None)

    # Save
    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    with open(output_path, 'w', encoding='utf-8') as f:
        json.dump({'documents': corpus}, f, indent=2)

    print(f"\nSaved {len(corpus)} documents to {output_path}")

    # Print stats
    kinds = defaultdict(int)
    for doc in corpus:
        kinds[doc['chunk_kind']] += 1
    print(f"\nFinal distribution:")
    for kind, count in sorted(kinds.items()):
        print(f"  {kind}: {count}")


if __name__ == '__main__':
    main()
