class HSEnv
  def initialize(script_dir)
    find_hs_env(script_dir)
    puts "hyper script home = #{@hs_home}, executable = #{@hs_exe}"
  end

  def find_hs_env(script_dir)
    path_script = File.join(script_dir, 'hs_env.sh')
    env = `bash #{path_script}`.delete("\n")
    @hs_home, @hs_exe = env.split(':')
  end

  def home
    @hs_home
  end

  def do_hs(arg, tags = [], path = @hs_home)
    tags = ['all'] if tags.length == 0
    tags_str = tags.join(',')
    `#{@hs_exe} --timeless -p #{path} -f #{tags_str} #{arg}`
  end
end

DIR = File.dirname(__FILE__)
HS_ENV = HSEnv.new(DIR)
