# frozen_string_literal: true

INPUT_PTRN = "tools/in/*.txt"
OUT_SCORE_FILE = "scores.txt"
OUT_DIR = "tools/out"

files_in = Dir.glob(INPUT_PTRN).sort

File.open(OUT_SCORE_FILE, 'w') do |fout|
    files_in.each do |fin|
        fout_each = File.join(OUT_DIR, File.basename(fin))
        File.delete(fout_each) if File.exist?(fout_each)
        # "Score = xxx" -> stderr -> stdout
        # other prints -> fout_each
        s = `../target/release/tester ../target/release/a < #{fin} 2>&1 1>#{fout_each}`

        # remove 8 characters ("Score = ")
        fout.write(s.slice(8, s.length))
    end
end
