require 'bundler'
Bundler.require(:default)
Dotenv.load()

$pluralkituri = 'https://api.pluralkit.me/v1/'
#make pluralkituri global
def GetSystemID(pluralkituri)
  uri = URI("#{pluralkituri}s")
  req = Net::HTTP::Get.new(uri)
  req['Authorization'] = (ENV['pluralkittoken'])
  res = Net::HTTP.start(uri.hostname, uri.port, :use_ssl => true) do |http| 
    http.request(req)
  end
  obj = JSON.parse(res.body)
  obj['id'] 
end

def GetSystemMembers(id, pluralkituri)
  memberlist = []
  uri = URI("#{pluralkituri}s/#{id}/members")
  req = Net::HTTP::Get.new(uri)
  req['Authorization'] = (ENV['pluralkittoken'])
  res = Net::HTTP.start(uri.hostname, uri.port, :use_ssl => true) do |http| 
    http.request(req)
  end
  systemmembers = JSON.parse(res.body)
  systemmembers.each do |member|
    memberlist.push(member["name"])
 end
 memberlist
 
  
  # puts res.body
end

def GetCurrentFronter(id, pluralkituri)
  uri = URI("#{pluralkituri}s/#{id}/fronters")
  req = Net::HTTP::Get.new(uri)
  req['Authorization'] = (ENV['pluralkittoken'])
  res = Net::HTTP.start(uri.hostname, uri.port, :use_ssl => true) do |http|
    http.request(req)
  end
  currentfronter = JSON.parse(res.body)
  currentfronter = currentfronter["members"].flatten
  currentfronter[0]["name"]
end


# id = GetSystemID(pluralkituri)
# sysmembernames = GetSystemMembers(id,pluralkituri)
# curfront = GetCurrentFronter(id,pluralkituri)
# puts curfront
