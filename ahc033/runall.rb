# frozen_string_literal: true

INPUT_PTRN = "tools/in/*.txt"
OUT_FILE = "out.txt"
MY_ANS_FILE = "myans.txt"

files_in = Dir.glob(INPUT_PTRN).sort

File.open(OUT_FILE, 'w') do |fout|
    files_in.each do |fin|
        File.delete(MY_ANS_FILE) if File.exist?(MY_ANS_FILE)
        s = `../target/release/a < #{fin} > #{MY_ANS_FILE} && ../target/release/vis #{fin} #{MY_ANS_FILE}`
        # remove 8 characters ("score = ")
        fout.write(s.slice(8, s.length))
    end
end
