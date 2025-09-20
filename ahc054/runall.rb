# frozen_string_literal: true

INPUT_PTRN = "tools/in/*.txt"
OUT_FILE = "scores.txt"
MY_ANS_FILE = "myans.txt"

files_in = Dir.glob(INPUT_PTRN).sort

File.open(OUT_FILE, 'w') do |fout|
    i = 0
    files_in.each do |fin|
        File.delete(MY_ANS_FILE) if File.exist?(MY_ANS_FILE)
        # "Score = xxx" -> stderr -> stdout
        # other prints -> stdout -> null
        s = `../target/release/tester ../target/release/a < #{fin} 2>&1 1>/dev/null`

        # remove 8 characters ("Score = ")
        fout.write(s.slice(8, s.length))
    end
end
