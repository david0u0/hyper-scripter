class HSEnv
  def initialize(script_dir)
    @hs_home = find_hs_path(script_dir)
    file =  File.open(File.join(@hs_home, '.hs_exe_path'))
    @hs_exe = file.read
    puts "hyper script home = #{@hs_home}, executable = #{@hs_exe}"
  end

  def find_hs_path(script_dir)
    path_script = File.join(script_dir, 'hs_path.sh')
    `bash #{path_script}`.delete("\n")
  end

  def do_hs(arg, tags = [], path = @hs_home)
    tags = ['all'] if tags.length == 0
    tags_str = tags.join(',')
    `#{@hs_exe} --timeless -p #{path} -f #{tags_str} #{arg}`
  end
end
