#!python3

# This script analyzes a VCF file and provides statistics about the INFO field keys and their values.
# It counts the total number of unique keys, categorical keys, numerical keys, and unique values.
# Usage: python3 info.py <input.vcf.gz>

import sys
import gzip

print(sys.argv[1])

def is_categorical(value):
    # Check if the value is a string and contains non-numeric characters
    if isinstance(value, str):
        return not value.replace('.', '', 1).isdigit()
    # If it's not a string, assume it's numeric
    return False


fi = gzip.open(sys.argv[1], 'r')
key_set = set()
categorical_set = set()
numerical_set = set()

value_map = {}

for line in fi:
    line = line.decode('utf-8').strip();
    if line[0] == '#':
        continue
    else:
        fields = line.split('\t')
        info = fields[7]
        info_fields = info.split(';')
        for info_field in info_fields:
            key_val = info_field.split('=')
            if len(key_val) == 2:
                key = key_val[0]
                value = key_val[1]
                if is_categorical(value):
                    categorical_set.add(key)
                else:
                    numerical_set.add(key)
                
                if key not in key_set:
                    key_set.add(key)
                    value_map[key] = set()
                value_map[key].add(value)
            else:
                categorical_set.add(info_field)
                key_set.add(info_field)

print(f"Total unique keys: {len(key_set)}")
print(f"Total categorical keys: {len(categorical_set)}")
print(f"Total numerical keys: {len(numerical_set)}")
print(f"Total unique values: {sum(len(v) for v in value_map.values())}")
print("Categorical keys:", sorted(categorical_set))
fi.close()