from sys import argv
import gzip
import argparse

def parse_args():
    parser = argparse.ArgumentParser(description="A sample script to process data.")
    parser.add_argument("-r", "--reference", type=str, help="Reference genome. Either 'hg19' or 'hg38'")
    parser.add_argument("-s", "--source", type=str, help="Gene source. Either 'refseq' or 'gencode'")
    return parser.parse_args()

def main(genome, source):
    print(genome, source)
    input = f"./{genome}/{genome}.gene.{source}.gtf.gz"
    output = f"./{genome}/{genome}.{source}.coordinates.csv"

    print(f"Reading {input} and writing {output}")

    fi = gzip.open(input, 'r')
    with open(output, 'w') as fo:
        for line in fi:
            tokens = line.decode('utf-8').strip().split('\t')
            chr = tokens[0]
            begin = tokens[3]
            end = tokens[4]
            info = tokens[8].split(';')

            if source == "refseq":
                gene_val = next((item for item in info if "gene_id" in item), None)
            else:
                gene_val = next((item for item in info if "gene_name" in item), None)

            if gene_val is None:
                print(f"Invalid gene entry found: {line}")
            else:
                gene = gene_val.split(' ')[1].replace("\"", "")
                fo.write(f"{chr},{begin},{end},{gene}\n")
    fi.close()

if __name__ == "__main__":
    args = parse_args()
    main(args.reference, args.source)