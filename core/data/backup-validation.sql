-- A backup is data, never executable schema. The caller checks tables, columns,
-- format markers and JSON syntax first. Return at most one invalid-data category.
SELECT 'annotations' FROM annotations WHERE
 typeof(edition)<>'text' OR edition='' OR typeof(book)<>'integer' OR book<0 OR
 typeof(chapter)<>'integer' OR chapter<0 OR typeof(verse)<>'integer' OR verse<0 OR
 json_type(payload,'$.note') IS NOT 'text' OR length(CAST(json_extract(payload,'$.note') AS BLOB))>32768 OR
 json_type(payload,'$.underline') NOT IN ('true','false') OR json_type(payload,'$.underline') IS NULL OR
 json_type(payload,'$.bookmark') NOT IN ('true','false') OR json_type(payload,'$.bookmark') IS NULL OR
 (json_type(payload,'$.ranges') IS NOT NULL AND json_type(payload,'$.ranges')<>'array') OR
 (json_extract(payload,'$.noteColor') IS NOT NULL AND json_extract(payload,'$.noteColor') NOT IN ('yellow','green','blue','pink')) OR
 (json_extract(payload,'$.noteStyle') IS NOT NULL AND json_extract(payload,'$.noteStyle') NOT IN ('plain','sticky'))
UNION ALL
SELECT 'settings' FROM settings WHERE
 (key='theme' AND (json_type(payload) IS NOT 'text' OR json_extract(payload,'$') NOT IN ('Default','Light','Dark'))) OR
 (key='language' AND (json_type(payload) IS NOT 'text' OR json_extract(payload,'$') NOT IN ('it','en'))) OR
 (key='bibleLanguage' AND (json_type(payload) IS NOT 'text' OR json_extract(payload,'$') NOT IN ('it','en','all'))) OR
 (key='fontSize' AND (json_type(payload) IS NOT 'integer' OR json_extract(payload,'$') NOT BETWEEN 14 AND 32)) OR
 (key='position' AND (json_type(payload,'$.edition') IS NOT 'text' OR json_type(payload,'$.book') IS NOT 'integer' OR json_extract(payload,'$.book')<0 OR json_type(payload,'$.chapter') IS NOT 'integer' OR json_extract(payload,'$.chapter')<0 OR (json_type(payload,'$.verse') IS NOT NULL AND (json_type(payload,'$.verse')<>'integer' OR json_extract(payload,'$.verse')<0))))
UNION ALL
SELECT 'editions' FROM editions WHERE typeof(book_count)<>'integer' OR book_count NOT BETWEEN 1 AND 100 OR json_type(payload) IS NOT 'array' OR json_array_length(payload)<>book_count OR length(CAST(payload AS BLOB))>33554432
UNION ALL
SELECT 'books' FROM editions e,json_each(e.payload) b WHERE json_type(b.value,'$.name') IS NOT 'text' OR json_type(b.value,'$.abbrev') IS NOT 'text' OR json_type(b.value,'$.chapters') IS NOT 'array' OR json_array_length(b.value,'$.chapters') NOT BETWEEN 1 AND 200
UNION ALL
SELECT 'chapters' FROM editions e,json_each(e.payload) b,json_each(b.value,'$.chapters') c WHERE c.type<>'array' OR json_array_length(c.value) NOT BETWEEN 1 AND 200
UNION ALL
SELECT 'verses' FROM editions e,json_each(e.payload) b,json_each(b.value,'$.chapters') c,json_each(c.value) v WHERE v.type<>'text' OR v.value=''
UNION ALL
SELECT 'organization' FROM note_organization WHERE NOT json_valid(tags) OR json_type(tags)<>'array' OR book<0 OR chapter<0 OR verse<0
UNION ALL
SELECT 'plans' FROM reading_plans WHERE id<1 OR json_type(payload,'$.edition') IS NOT 'text' OR json_type(payload,'$.title') IS NOT 'text' OR json_type(payload,'$.daily') IS NOT 'integer' OR json_extract(payload,'$.daily') NOT BETWEEN 1 AND 12 OR json_type(payload,'$.start') IS NOT 'text' OR json_type(payload,'$.resume') IS NOT 'object'
UNION ALL
SELECT 'progress' FROM reading_progress p WHERE p.chapter_index<0 OR NOT EXISTS(SELECT 1 FROM reading_plans r WHERE r.id=p.plan_id)
LIMIT 1;
