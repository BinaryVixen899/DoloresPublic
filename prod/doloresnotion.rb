require_relative 'dolorespluralkit'
require 'bundler'
Bundler.require(:default)
Dotenv.load()


def convertaltertojson(alter)
 ruby = {"heading_1" => {
      "text" =>
      [
        {
          "type" => "text",
          "text" => {
            "content" => "#{alter}",
          }
        }
      ]
    }
  }
  json = JSON.pretty_generate(ruby)
  # thisfunction.to_json
  #Use json.generate
rescue => e 
  puts "could not convert alter to JSON"
end

def getblocksforpage(blockid)
  uri = URI("https://api.notion.com/v1/blocks/#{blockid}")  
  req = Net::HTTP::Get.new(uri)
  req['Notion-Version'] = '2021-08-16'
  req['Authorization'] = (ENV['NOTION_API_KEY']).to_s
  res = Net::HTTP.start(uri.hostname, uri.port, :use_ssl => true ) do |http|
    http.request(req)
  end

  response = JSON.parse(res.body)
  puts response
  if response['has_children']
    puts "Oh! Congratulations"
    response = getblockschildren(blockid)
    return response
  else
    puts "This does not have children"
    return response
  end

rescue => e
  puts "couldn't get blocks uwu"
  puts e
end

def getblockschildren(blockid)
  uri = URI("https://api.notion.com/v1/blocks/#{blockid}/children?page_size=100")
  req = Net::HTTP::Get.new(uri)
  req['Notion-Version'] = '2021-08-16'
  req['Authorization'] = (ENV['NOTION_API_KEY']).to_s
  res = Net::HTTP.start(uri.hostname, uri.port, :use_ssl => true) do |http|
    http.request(req)
  end
  response = JSON.parse(res.body)

rescue => e
  puts "couldn't find block children uwu"
end

def GetNotionPageFronter(response)
  
  puts response
  puts response['object']
  binding.pry
  test = response['results'][2]['heading_1']['text'][0]['plain_text']
  puts test
  test

rescue => e 
  puts "failed to get notion page fronter uwu"
end

def UpdateFrontingAlter(alter=nil)
  #addalter as optional argument 
  id = GetSystemID($pluralkituri)
  if alter == nil
    somejson = convertaltertojson(GetCurrentFronter(id, $pluralkituri))
  else
    alter = alter
    somejson = convertaltertojson(alter)
  end
  
  block_id = 'REDACTED'
  uri = URI("https://api.notion.com/v1/blocks/#{block_id}")
  req = Net::HTTP::Patch.new(uri)
  req['Notion-Version'] = '2021-08-16'
  req['Authorization'] = (ENV['NOTION_API_KEY']).to_s
  req.body = somejson
  req.content_type = "application/json"
  pp req.body
  res = Net::HTTP.start(uri.hostname, uri.port, :use_ssl => true) do |http|
    http.request(req)
  end
rescue => e
  puts "failed to update fronting alter uwu"
end

def GetSpecies(blockid='REDACTED')
  # Takes a blockID if one is given and returns either the current species or nil 
  uri = URI("https://api.notion.com/v1/blocks/#{blockid}/children")
  req = Net::HTTP::Get.new(uri)
  req['Notion-Version'] = '2021-08-16'
  req['Authorization'] = (ENV['NOTION_API_KEY']).to_s
  res = Net::HTTP.start(uri.hostname, uri.port, :use_ssl => true) do |http|
    http.request(req)
  end
  if res.code == '200'
    obj = JSON.parse(res.body)
    obj['results'][1]['heading_1']['text'][0]['plain_text']  
  else
    # logger.info("Response Code #{res.code} received!")
    nil
  end
  rescue => e
    puts "failed to get species uwu"
    'Kitsune'
end

def GetPronouns(blockid='REDACTED')
  uri = URI("https://api.notion.com/v1/blocks/#{blockid}/children")
  req = Net::HTTP::Get.new(uri)
  req['Notion-Version'] = '2021-08-16'
  req['Authorization'] = (ENV['NOTION_API_KEY']).to_s
  res = Net::HTTP.start(uri.hostname, uri.port, :use_ssl => true) do |http|
    http.request(req)
  end
  # logger.info(puts res.body)
  if res.code == '200'
    obj = JSON.parse(res.body)
    obj['results'][3]['heading_1']['text'][0]['plain_text']
  else
    'She/Her'
  end
  rescue => e 
    puts "failed to get pronouns uwu"
    'She/Her'
end
# GetNotionPageFronter(getblocksforpage('REDACTED'))





