Import("env")
import gzip
import os

def gzip_webfiles(source, target, env):
    """Gzip web files before building the filesystem image."""
    data_dir = os.path.join(env.subst("$PROJECT_DIR"), "data")

    if not os.path.isdir(data_dir):
        print("  No data/ directory found, skipping gzip")
        return

    extensions = ['.html', '.js', '.css']

    for filename in os.listdir(data_dir):
        filepath = os.path.join(data_dir, filename)

        if not os.path.isfile(filepath):
            continue
        if filename.endswith('.gz'):
            continue

        _, ext = os.path.splitext(filename)
        if ext not in extensions:
            continue

        gz_path = filepath + '.gz'

        # Check if gzip is up to date
        if os.path.exists(gz_path):
            if os.path.getmtime(gz_path) >= os.path.getmtime(filepath):
                print(f"  Skipping {filename} (up to date)")
                continue

        # Read and gzip file
        print(f"  Gzipping {filename}")
        with open(filepath, 'rb') as f_in:
            content = f_in.read()

        with gzip.open(gz_path, 'wb', compresslevel=9) as f_out:
            f_out.write(content)

        # Show size reduction
        orig_size = len(content)
        gz_size = os.path.getsize(gz_path)
        reduction = (1 - gz_size / orig_size) * 100
        print(f"    {orig_size} -> {gz_size} bytes ({reduction:.1f}% reduction)")

# Run before building filesystem
env.AddPreAction("$BUILD_DIR/littlefs.bin", gzip_webfiles)
