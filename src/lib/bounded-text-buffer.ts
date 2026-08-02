const chunkTargetBytes = 16 * 1024;
const encoder = new TextEncoder();
const decoder = new TextDecoder();

interface TextChunk {
  text: string;
  bytes: number;
}

export class BoundedTextBuffer {
  private chunks: TextChunk[] = [];
  private head = 0;
  private retainedBytes = 0;

  constructor(private readonly maxBytes: number) {}

  append(content: string) {
    if (!content || this.maxBytes <= 0) return;
    const contentBytes = encoder.encode(content).byteLength;
    const tail = this.chunks.at(-1);
    if (tail && this.chunks.length > this.head && tail.bytes + contentBytes <= chunkTargetBytes) {
      tail.text += content;
      tail.bytes += contentBytes;
    } else {
      this.chunks.push({ text: content, bytes: contentBytes });
    }
    this.retainedBytes += contentBytes;
    this.trimToLimit();
  }

  snapshot() {
    return this.chunks.slice(this.head).map((chunk) => chunk.text).join("");
  }

  get byteLength() {
    return this.retainedBytes;
  }

  get chunkCount() {
    return this.chunks.length - this.head;
  }

  private trimToLimit() {
    while (this.retainedBytes > this.maxBytes && this.head < this.chunks.length) {
      const excess = this.retainedBytes - this.maxBytes;
      const front = this.chunks[this.head];
      if (front.bytes <= excess) {
        this.retainedBytes -= front.bytes;
        this.head += 1;
        continue;
      }
      const encoded = encoder.encode(front.text);
      let start = excess;
      while (start < encoded.length && (encoded[start] & 0xc0) === 0x80) start += 1;
      front.text = decoder.decode(encoded.subarray(start));
      front.bytes = encoded.length - start;
      this.retainedBytes -= start;
    }
    if (this.head >= 64 && this.head * 2 >= this.chunks.length) {
      this.chunks = this.chunks.slice(this.head);
      this.head = 0;
    }
  }
}
