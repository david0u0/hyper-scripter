class HSEnv
  def initialize(script_dir)
    @hs_path = find_hs_path(script_dir)
    puts "hyper script path = #{@hs_path}"
  end

  def find_hs_path(script_dir)
    path_script = File.join(script_dir, '.hs_path.sh')
    `bash #{path_script}`.delete("\n")
  end

  def do_hs(arg, tags = [], path = @hs_path)
    tags = ['all'] if tags.length == 0
    tags_str = tags.join(',')
    `hs -p #{path} -t #{tags_str} #{arg}`
  end
end
