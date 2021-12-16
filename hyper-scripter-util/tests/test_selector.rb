require_relative '../util/selector'

class Option
  def initialize(content, number)
    @content = content.strip
    @number = number * 2
  end

  def to_s
    "#{@number}-#{@content}"
  end
  attr_reader :number, :content
end

opts = %w[a b c d e f g]
opts = opts.map.with_index { |opt, i| Option.new(opt, i) }

mode = :normal
selector = Selector.new(offset: 2)
selector.load(opts.clone)

selector.register_keys(%w[A], ->(_, obj) { puts obj }, recur: true)
selector.register_keys(%w[p], ->(_, obj) { puts obj.content }, recur: true)
selector.register_keys(%w[P], ->(_, _obj) { mode = :print_all })

selector.register_keys(%w[l], ->(_, _) { puts '====' }, recur: true)
selector.register_keys_virtual(%w[l], ->(_, _, _) { puts '====' }, recur: true)

selector.register_keys_virtual(%w[p], lambda { |_, _, objs|
  objs.each do |obj|
    puts "range print #{obj.content}"
  end
}, recur: true)
selector.register_keys_virtual(%w[P], ->(_, _, _) { mode = :print_all_virtual })

selector.register_keys(%w[d], lambda { |pos, obj|
  puts "delete #{obj.content}"
  opts.delete_at(pos)
  selector.load(opts.clone)
}, recur: true)
selector.register_keys_virtual(%w[d], lambda { |min, _, objs|
  objs.each do |obj|
    puts "range delete #{obj.content}"
    opts.delete_at(min)
  end
  selector.load(opts.clone)
  selector.exit_virtual
}, recur: true)

res = begin
  selector.run(sequence: ARGV[0])
rescue Selector::Empty
  puts 'empty'
  exit
rescue Selector::Quit
  puts 'quit'
  exit
end

case mode
when :normal
  puts res.content
when :print_all
  opts.each { |obj| puts obj.content }
when :print_all_virtual
  res.options.each { |obj| puts obj.content }
end
