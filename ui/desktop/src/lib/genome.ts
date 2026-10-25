// Parse common genome annotation formats (BED / bedGraph / GFF3 / GTF / VCF)
// into a uniform feature model for the native track viewer. Pure and offline —
// no reference genome, no network — works on the file alone. All coordinates
// are normalized to 1-based inclusive so ruler and tooltips are consistent.

export type GenomeFormat = 'bed' | 'bedgraph' | 'gff' | 'gtf' | 'vcf';

export interface GenomeFeature {
  chrom: string;
  /** 1-based inclusive start. */
  start: number;
  /** 1-based inclusive end. */
  end: number;
  name?: string;
  strand?: '+' | '-';
  /** Feature type (GFF/GTF col 3), e.g. "gene", "exon". */
  type?: string;
  score?: number;
}

export interface Contig {
  name: string;
  /** Smallest feature start on this contig (1-based). */
  min: number;
  /** Largest feature end on this contig (1-based). */
  max: number;
  count: number;
}

export interface GenomeData {
  format: GenomeFormat;
  features: GenomeFeature[];
  contigs: Contig[];
  /** True if the file had more features than the parse cap. */
  truncated: boolean;
}

/** Cap parsed features so a huge annotation file cannot exhaust browser memory. */
const MAX_FEATURES = 50_000;

/** The annotation format for a file extension, or null if not a track format. */
export function genomeFormat(ext: string): GenomeFormat | null {
  switch (ext.toLowerCase()) {
    case 'bed':
      return 'bed';
    case 'bedgraph':
    case 'bdg':
      return 'bedgraph';
    case 'gff':
    case 'gff3':
      return 'gff';
    case 'gtf':
      return 'gtf';
    case 'vcf':
      return 'vcf';
    default:
      return null;
  }
}

const strandOf = (s: string): '+' | '-' | undefined =>
  s === '+' ? '+' : s === '-' ? '-' : undefined;

const numOr = (s: string | undefined): number | undefined => {
  if (s === undefined || s === '' || s === '.') return undefined;
  const n = Number(s);
  return Number.isFinite(n) ? n : undefined;
};

/** GFF3 attributes: `ID=gene1;Name=BRCA1`. Returns the first present key's value. */
function gffName(attrs: string): string | undefined {
  const map = new Map<string, string>();
  for (const kv of attrs.split(';')) {
    const eq = kv.indexOf('=');
    if (eq > 0) map.set(kv.slice(0, eq).trim().toLowerCase(), decodeURIComponent(kv.slice(eq + 1).trim()));
  }
  for (const k of ['name', 'gene_name', 'gene', 'id']) {
    const v = map.get(k);
    if (v) return v;
  }
  return undefined;
}

/** GTF attributes: `gene_id "X"; gene_name "BRCA1";`. */
function gtfName(attrs: string): string | undefined {
  const map = new Map<string, string>();
  for (const m of attrs.matchAll(/(\w+)\s+"([^"]*)"/g)) map.set(m[1].toLowerCase(), m[2]);
  for (const k of ['gene_name', 'gene_id', 'transcript_id']) {
    const v = map.get(k);
    if (v) return v;
  }
  return undefined;
}

function parseLine(format: GenomeFormat, line: string): GenomeFeature | null {
  const f = format === 'bed' || format === 'bedgraph' ? line.split(/\s+/) : line.split('\t');
  if (f.length < 3) return null;

  switch (format) {
    case 'bed': {
      // chrom start end [name score strand ...]
      // BED coords are 0-based half-open [start, end) → convert to 1-based inclusive.
      const start0 = Number(f[1]);
      const end0 = Number(f[2]);
      if (!Number.isFinite(start0) || !Number.isFinite(end0) || end0 <= start0) return null;
      return {
        chrom: f[0],
        start: start0 + 1,
        end: end0,
        name: f[3] && f[3] !== '.' ? f[3] : undefined,
        score: numOr(f[4]),
        strand: f[5] ? strandOf(f[5]) : undefined,
      };
    }
    case 'bedgraph': {
      // chrom start end value
      const start0 = Number(f[1]);
      const end0 = Number(f[2]);
      if (!Number.isFinite(start0) || !Number.isFinite(end0) || end0 <= start0) return null;
      return {
        chrom: f[0],
        start: start0 + 1,
        end: end0,
        score: numOr(f[3]),
      };
    }
    case 'gff':
    case 'gtf': {
      // seqid source type start end score strand phase attributes
      // 1-based inclusive already.
      if (f.length < 8) return null;
      const start = Number(f[3]);
      const end = Number(f[4]);
      if (!Number.isFinite(start) || !Number.isFinite(end) || end < start) return null;
      const attrs = f[8] ?? '';
      return {
        chrom: f[0],
        start,
        end,
        type: f[2] && f[2] !== '.' ? f[2] : undefined,
        score: numOr(f[5]),
        strand: strandOf(f[6]),
        name: format === 'gff' ? gffName(attrs) : gtfName(attrs),
      };
    }
    case 'vcf': {
      // CHROM POS ID REF ALT QUAL FILTER INFO ...
      // 1-based single-base locus; indel end estimated from REF length.
      const pos = Number(f[1]);
      if (!Number.isFinite(pos) || pos < 1) return null;
      const ref = f[3] ?? '';
      const alt = f[4] ?? '';
      const name = f[2] && f[2] !== '.' ? f[2] : `${ref}>${alt}`;
      const end = pos + Math.max(0, ref.length - 1);
      return {
        chrom: f[0],
        start: pos,
        end,
        name,
        type: 'variant',
        score: numOr(f[5]),
      };
    }
  }
}

/** Natural-sort comparator for contig names (chr1 < chr2 < chr10 < chrX < chrY < chrM). */
export function compareContigs(a: string, b: string): number {
  return a.localeCompare(b, undefined, { numeric: true, sensitivity: 'base' });
}

export function parseGenome(text: string, format: GenomeFormat): GenomeData {
  const features: GenomeFeature[] = [];
  const contigsMap = new Map<string, { min: number; max: number; count: number }>();
  let truncated = false;

  for (const raw of text.split(/\r?\n/)) {
    const line = raw.trim();
    if (!line || line.startsWith('#') || line.startsWith('track ') || line.startsWith('browser ')) {
      continue;
    }
    const feat = parseLine(format, line);
    if (!feat) continue;

    if (features.length >= MAX_FEATURES) {
      truncated = true;
      break;
    }
    features.push(feat);

    const c = contigsMap.get(feat.chrom);
    if (c) {
      if (feat.start < c.min) c.min = feat.start;
      if (feat.end > c.max) c.max = feat.end;
      c.count += 1;
    } else {
      contigsMap.set(feat.chrom, { min: feat.start, max: feat.end, count: 1 });
    }
  }

  const contigs: Contig[] = [...contigsMap.entries()]
    .map(([name, s]) => ({ name, min: s.min, max: s.max, count: s.count }))
    .sort((a, b) => compareContigs(a.name, b.name));

  return { format, features, contigs, truncated };
}

/** Assign each feature to the lowest row index that does not overlap any earlier feature in that row. */
export function packRows(features: GenomeFeature[]): number[] {
  const rowEnds: number[] = [];
  return features.map((f) => {
    let placed = -1;
    for (let r = 0; r < rowEnds.length; r++) {
      if (rowEnds[r] < f.start) {
        rowEnds[r] = f.end;
        placed = r;
        break;
      }
    }
    if (placed === -1) {
      placed = rowEnds.length;
      rowEnds.push(f.end);
    }
    return placed;
  });
}

/** Format a base-pair count into a readable string: 1,234 bp, 12.3 kb, 1.2 Mb. */
export function formatBp(bp: number): string {
  const abs = Math.abs(bp);
  if (abs >= 1_000_000) return `${(bp / 1_000_000).toFixed(2)} Mb`;
  if (abs >= 1_000) return `${(bp / 1_000).toFixed(1)} kb`;
  return `${bp.toLocaleString()} bp`;
}
